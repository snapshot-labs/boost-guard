use crate::routes::boost_query::BoostQueryBoostStrategy;
use crate::routes::boost_query::BoostQueryBoostStrategyEligibility;
use crate::signatures::ClaimConfig;
use crate::State;
use crate::{ServerError, HUB_URL, SUBGRAPH_URL};
use ::axum::extract::Json;
use axum::response::IntoResponse;
use axum::Extension;
use ethers::types::Address;
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use std::time::SystemTime;

// TODO: check with BIG voting power (f64 precision?)
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateVouchersResponse {
    pub signature: String,
    pub reward: String,
    pub chain_id: String,
    pub boost_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetRewardsResponse {
    pub reward: String,
    pub chain_id: String,
    pub boost_id: String,
}

impl From<RewardInfo> for GetRewardsResponse {
    fn from(reward_info: RewardInfo) -> Self {
        Self {
            reward: reward_info.reward,
            chain_id: reward_info.chain_id,
            boost_id: reward_info.boost_id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RewardInfo {
    pub voter_address: String,
    pub reward: String,
    pub chain_id: String,
    pub boost_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QueryParams {
    pub proposal_id: String,
    pub voter_address: String,
    pub boosts: Vec<(String, String)>, // Vec<(boost_id, chain_id)>
}

type Bytes = Address;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/subgraph_schema.json",
    query_path = "src/graphql/boost_query.graphql",
    response_derives = "Debug"
)]
struct BoostQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/hub_schema.graphql",
    query_path = "src/graphql/proposal_query.graphql",
    response_derives = "Debug"
)]
struct ProposalQuery;

type Any = u8;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/hub_schema.graphql",
    query_path = "src/graphql/vote_query.graphql",
    response_derives = "Debug"
)]
struct VotesQuery;

// Different strategies supported
#[derive(Debug)]
enum BoostStrategy {
    Proposal, // Boost a specific proposal
}

impl TryFrom<&str> for BoostStrategy {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "proposal" => Ok(BoostStrategy::Proposal),
            _ => Err("Invalid strategy"),
        }
    }
}

#[allow(dead_code)] // needed for `strategy` field
#[derive(Debug)]
pub struct BoostInfo {
    strategy: BoostStrategy,
    params: BoostParams,
    pool_size: u128,
    decimals: u8,
}

impl TryFrom<boost_query::BoostQueryBoost> for BoostInfo {
    type Error = &'static str;

    fn try_from(value: boost_query::BoostQueryBoost) -> Result<Self, Self::Error> {
        let strategy: BoostQueryBoostStrategy = value.strategy.ok_or("heyhey")?;
        let name = strategy.name.as_str();
        let strategy_type = BoostStrategy::try_from(name)?;

        match strategy_type {
            BoostStrategy::Proposal => {
                let eligibility = BoostEligibility::try_from(strategy.eligibility)?;

                let distribution =
                    DistributionType::from_str(strategy.distribution.type_.as_str())?;

                let bp = BoostParams {
                    version: strategy.version,
                    proposal: strategy.proposal,
                    eligibility,
                    distribution,
                };

                let pool_size = value
                    .pool_size
                    .parse()
                    .map_err(|_| "failed to parse pool size")?;
                let decimals = value
                    .token
                    .decimals
                    .parse()
                    .map_err(|_| "failed to parse decimals")?;

                Ok(Self {
                    strategy: strategy_type,
                    params: bp,
                    pool_size,
                    decimals,
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct BoostParams {
    pub version: String,
    pub proposal: String,
    pub eligibility: BoostEligibility,
    pub distribution: DistributionType,
}

#[derive(Debug, Copy, Clone)]
pub enum BoostEligibility {
    Incentive, // Everyone who votes is eligible, regardless of choice
    Bribe(u8), // Only those who voted for the specific choice are eligible
}

impl TryFrom<BoostQueryBoostStrategyEligibility> for BoostEligibility {
    type Error = &'static str;

    fn try_from(value: BoostQueryBoostStrategyEligibility) -> Result<Self, Self::Error> {
        match value.type_.as_str() {
            "incentive" => Ok(BoostEligibility::Incentive),
            "bribe" => {
                let choice = value
                    .choice
                    .ok_or("missing choice")?
                    .try_into()
                    .map_err(|_| "failed to parse choice")?;
                Ok(BoostEligibility::Bribe(choice))
            }
            _ => Err("invalid eligibility"),
        }
    }
}

#[derive(Debug)]
pub enum DistributionType {
    Weighted(Option<u128>), // The option represents the maximum amount of tokens that can be rewarded.
    Even,
}

impl FromStr for DistributionType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "weighted" => Ok(DistributionType::Weighted(None)),
            "even" => Ok(DistributionType::Even),
            _ => Err("invalid distribution"),
        }
    }
}

#[derive(Debug)]
struct Vote {
    voting_power: f64,
    choice: u8,
}

#[derive(Debug)]
struct Proposal {
    type_: String,
    score: f64,
    end: u64,
    votes: u64,
}

impl TryFrom<proposal_query::ProposalQueryProposal> for Proposal {
    type Error = ServerError;

    fn try_from(proposal: proposal_query::ProposalQueryProposal) -> Result<Self, Self::Error> {
        let proposal_type = proposal.type_.ok_or("missing proposal type from the hub")?;
        let proposal_score = proposal
            .scores_total
            .ok_or("missing proposal score from the hub")?;
        let proposal_end = proposal.end.try_into()?;
        let votes = proposal
            .votes
            .ok_or("missing votes from the hub")?
            .try_into()
            .map_err(|_| ServerError::ErrorString("failed to parse votes".to_string()))?;

        Ok(Proposal {
            type_: proposal_type,
            score: proposal_score,
            end: proposal_end,
            votes,
        })
    }
}

async fn get_rewards_inner(
    state: &State,
    p: serde_json::Value,
) -> Result<Vec<RewardInfo>, ServerError> {
    let request: QueryParams = serde_json::from_value(p)?;

    let proposal = get_proposal_info(&state.client, &request.proposal_id).await?;
    let vote = get_vote_info(&state.client, &request.voter_address, &request.proposal_id).await?;

    validate_end_time(proposal.end)?; // We do not need to validate start_time because the smart-contract will do it anyway
    validate_type(&proposal.type_)?;

    let mut response = Vec::with_capacity(request.boosts.len());
    for (boost_id, chain_id) in request.boosts {
        let Ok(boost_info) = get_boost_info(&state.client, &boost_id).await else {
            continue;
        };
        let pool: u128 = boost_info.pool_size;
        let decimals: u8 = boost_info.decimals;

        if boost_info.params.proposal != request.proposal_id {
            continue;
        }

        if validate_choice(vote.choice, boost_info.params.eligibility).is_err() {
            continue;
        }
        // TODO: check cap

        let voting_power = vote.voting_power * 10f64.powi(decimals as i32);
        let score = proposal.score * 10f64.powi(decimals as i32);
        let reward = compute_user_reward(
            pool,
            voting_power as u128,
            score as u128,
            boost_info.params.distribution,
            proposal.votes,
        );

        response.push(RewardInfo {
            voter_address: request.voter_address.clone(),
            reward: reward.to_string(),
            chain_id,
            boost_id,
        });
    }

    Ok(response)
}

pub async fn handle_create_vouchers(
    Extension(state): Extension<State>,
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    let reward_infos = get_rewards_inner(&state, p).await?;

    let mut response = Vec::with_capacity(reward_infos.len());
    for reward_info in reward_infos {
        let Ok(claim_cfg) = ClaimConfig::try_from(&reward_info) else {
            continue;
        };
        let Ok(signature) = claim_cfg.create_signature(&state.wallet) else {
            continue;
        };

        response.push(CreateVouchersResponse {
            signature: format!("0x{}", signature),
            reward: reward_info.reward,
            chain_id: reward_info.chain_id,
            boost_id: reward_info.boost_id,
        });
    }
    Ok(Json(response))
}

pub async fn handle_get_rewards(
    Extension(state): Extension<State>,
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    let response = get_rewards_inner(&state, p)
        .await?
        .into_iter()
        .map(GetRewardsResponse::from)
        .collect::<Vec<_>>(); // todo

    Ok(Json(response))
}

pub async fn handle_health() -> Result<impl IntoResponse, ServerError> {
    Ok(axum::response::Html("Healthy!"))
}

async fn get_proposal_info(
    client: &reqwest::Client,
    proposal_id: &str,
) -> Result<Proposal, ServerError> {
    let variables = proposal_query::Variables {
        id: proposal_id.to_owned(),
    };

    let request_body = ProposalQuery::build_query(variables);

    let res = client
        .post(HUB_URL.as_str())
        .json(&request_body)
        .send()
        .await?;
    let response_body: GraphQLResponse<proposal_query::ResponseData> = res.json().await?;
    let proposal_query: proposal_query::ProposalQueryProposal = response_body
        .data
        .ok_or("missing data from the hub")?
        .proposal
        .ok_or("missing proposal data from the hub")?;
    Proposal::try_from(proposal_query)
}

async fn get_boost_info(
    client: &reqwest::Client,
    boost_id: &str,
) -> Result<BoostInfo, ServerError> {
    let variables = boost_query::Variables {
        id: boost_id.to_owned(),
    };

    let request_body = BoostQuery::build_query(variables);

    let res = client
        .post(SUBGRAPH_URL.as_str())
        .json(&request_body)
        .send()
        .await?;
    let response_body: GraphQLResponse<boost_query::ResponseData> = res.json().await?;
    let boost_query = response_body.data.ok_or("missing data from the hub")?;

    let boost = boost_query.boost.ok_or("missing boost from the hub")?;
    Ok(BoostInfo::try_from(boost)?)
}

async fn get_vote_info(
    client: &reqwest::Client,
    voter_address: &str,
    proposal_id: &str,
) -> Result<Vote, ServerError> {
    let variables = votes_query::Variables {
        voter: voter_address.to_owned(),
        proposal: proposal_id.to_owned(),
    };

    let request_body = VotesQuery::build_query(variables);

    let res = client
        .post(HUB_URL.as_str())
        .json(&request_body)
        .send()
        .await?;
    let response_body: GraphQLResponse<votes_query::ResponseData> = res.json().await?;
    let votes = response_body
        .data
        .ok_or("missing data from the hub")?
        .votes
        .ok_or("missing votes from the hub")?;

    let vote = votes
        .into_iter()
        .next()
        .ok_or("voter has not voted for this proposal")?
        .ok_or("missing first vote from the hub?")?;

    Ok(Vote {
        voting_power: vote.vp.ok_or("missing vp from the hub")?,
        choice: vote.choice,
    })
}

fn compute_user_reward(
    pool: u128,
    voting_power: u128,
    proposal_score: u128,
    cap: DistributionType,
    votes: u64,
) -> u128 {
    match cap {
        DistributionType::Even => pool / (votes as u128),
        DistributionType::Weighted(cap) => {
            let reward = voting_power * pool / proposal_score;
            if let Some(limit) = cap {
                if reward > limit {
                    return limit;
                }
            }

            reward
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_user_reward, DistributionType};

    #[test]
    fn full_vp_no_cap() {
        let voting_power = 100;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 1;
        let cap = DistributionType::Weighted(None);

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 100);
    }

    #[test]
    fn full_vp_with_cap() {
        let voting_power = 100;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 1;
        let cap = DistributionType::Weighted(Some(50));

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 50);
    }

    #[test]
    fn full_vp_with_cap_not_reached() {
        let voting_power = 100;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 1;
        let cap = DistributionType::Weighted(Some(110));

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 100);
    }

    #[test]
    fn half_vp_no_cap() {
        let voting_power = 50;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 2;
        let cap = DistributionType::Weighted(None);

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 50);
    }

    #[test]
    fn half_vp_with_cap() {
        let voting_power = 50;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 2;
        let cap = DistributionType::Weighted(Some(25));

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 25);
    }

    #[test]
    fn half_vp_with_cap_not_reached() {
        let voting_power = 50;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 2;
        let cap = DistributionType::Weighted(Some(75));

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 50);
    }

    #[test]
    fn third_vp_no_cap() {
        let voting_power = 10;
        let proposal_score = 30;
        let pool_size = 100;
        let votes = 3;
        let cap = DistributionType::Weighted(None);

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 33);
    }

    #[test]
    fn third_vp_with_cap() {
        let voting_power = 10;
        let proposal_score = 30;
        let pool_size = 100;
        let votes = 3;
        let cap = DistributionType::Weighted(Some(18));

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 18);
    }

    #[test]
    fn third_vp_with_cap_not_reached() {
        let voting_power = 10;
        let proposal_score = 30;
        let pool_size = 100;
        let votes = 3;
        let cap = DistributionType::Weighted(Some(50));

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 33);
    }

    #[test]
    fn even_distribution_two() {
        let voting_power = 100;
        let proposal_score = 100;
        let pool_size = 100;
        let votes = 2;
        let cap = DistributionType::Even;

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 50);
    }

    #[test]
    fn even_distribution_three() {
        let voting_power = 10;
        let proposal_score = 30;
        let pool_size = 100;
        let votes = 3;
        let cap = DistributionType::Even;

        let reward = compute_user_reward(pool_size, voting_power, proposal_score, cap, votes);

        assert_eq!(reward, 33);
    }
}

fn validate_end_time(end: u64) -> Result<(), ServerError> {
    let current_timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap() // Safe to unwrap because we are sure that the current time is after the UNIX_EPOCH
        .as_secs();
    if current_timestamp < end {
        Err(ServerError::ErrorString(format!(
            "proposal has not ended yet: {end:} > {current_timestamp:}",
        )))
    } else {
        Ok(())
    }
}

fn validate_type(type_: &str) -> Result<(), ServerError> {
    if (type_ != "single-choice") && (type_ != "basic") {
        Err(ServerError::ErrorString(format!(
            "`{type_:}` proposals are not eligible for boosting"
        )))
    } else {
        Ok(())
    }
}

fn validate_choice(choice: u8, boost_eligibility: BoostEligibility) -> Result<(), ServerError> {
    match boost_eligibility {
        BoostEligibility::Incentive => Ok(()),
        BoostEligibility::Bribe(boosted_choice) => {
            if choice != boosted_choice {
                Err(ServerError::ErrorString(format!(
                    "voter voted {:} but needed to vote {} to be eligible",
                    choice, boosted_choice
                )))
            } else {
                Ok(())
            }
        }
    }
}
