use crate::signatures::ClaimConfig;
use crate::State;
use crate::{ServerError, HUB_URL};
use ::axum::extract::Json;
use axum::response::IntoResponse;
use axum::Extension;
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;

// TODO: check with BIG voting power (f64 precision?)

#[derive(Debug, Deserialize, Serialize)]
pub struct ForwardParams {
    pub to: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateVoucherResponse {
    // TODO: should we include ID of request?
    pub signature: String,
    pub chain_id: String,
    pub boost_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetRewardsResponse {
    // TODO: should we include ID of request?
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

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/proposal_query.graphql",
    response_derives = "Debug"
)]
struct ProposalQuery;

// TODO: only works for basic ? idk
type Any = u8;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/vote_query.graphql",
    response_derives = "Debug"
)]
struct VotesQuery;

// todo: docs
// todo: check that proposal has ended
pub async fn create_voucher_handler(
    Extension(state): Extension<State>,
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    let request: QueryParams = serde_json::from_value(p)?;

    let proposal = get_proposal_info(&state.client, &request.proposal_id).await?;
    let vote = get_vote_info(&state.client, &request.voter_address, &request.proposal_id).await?;

    validate_end_time(proposal.end)?;
    validate_type(&proposal.type_)?;

    let mut response = Vec::with_capacity(request.boosts.len());
    for (boost_id, chain_id) in request.boosts {
        let cap = None; // TODO: get this from ... somewhere?
        let boosted_choice = BoostStrategy::Incentive; // TODO: get this from ... somewhere?
        let pool: u128 = 100; // TODO: get this from... somewhere?
        let decimals: i32 = 18; // TODO: get this from... somewhere?

        validate_choice(vote.choice, boosted_choice)?;
        // TODO: check cap

        let voting_power = vote.voting_power * 10f64.powi(decimals);
        let reward = compute_user_reward(pool, voting_power as u128, proposal.score, cap);

        let signature = ClaimConfig::new(&boost_id, &chain_id, &request.voter_address, reward)?
            .create_signature(&state.wallet)?; // TODO: decide if we should error the whole request or only this specific boost?
        response.push(CreateVoucherResponse {
            signature: format!("0x{}", signature),
            chain_id,
            boost_id,
        });
    }

    Ok(Json(response))
}

// TODO: unify get_rewards_handle and create_voucher_handler

// todo: check that proposal has ended
pub async fn get_rewards_handler(
    Extension(state): Extension<State>,
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    let request: QueryParams = serde_json::from_value(p)?;

    let proposal = get_proposal_info(&state.client, &request.proposal_id).await?;
    let vote = get_vote_info(&state.client, &request.voter_address, &request.proposal_id).await?;

    validate_end_time(proposal.end)?;
    validate_type(&proposal.type_)?;

    let mut response = Vec::with_capacity(request.boosts.len());
    for (boost_id, chain_id) in request.boosts {
        let cap = None; // TODO: get this from ... somewhere?
        let boosted_choice = BoostStrategy::Incentive; // TODO: get this from ... somewhere?
        let pool: u128 = 100; // TODO: get this from... somewhere?
        let decimals: i32 = 18; // TODO: get this from... somewhere?

        validate_choice(vote.choice, boosted_choice)?;
        // TODO: check cap

        let voting_power = vote.voting_power * 10f64.powi(decimals);
        let reward = compute_user_reward(pool, voting_power as u128, proposal.score, cap);

        response.push(GetRewardsResponse {
            reward: reward.to_string(),
            chain_id,
            boost_id,
        });
    }

    Ok(Json(response))
}

fn compute_user_reward(
    pool: u128,
    voting_power: u128,
    proposal_score: u128,
    cap: Option<f64>,
) -> u128 {
    let reward = voting_power * pool / proposal_score;

    if let Some(_cap) = cap {
        todo!("implement cap");
    } else {
        reward
    }
}

fn validate_end_time(end: u64) -> Result<(), ServerError> {
    let current_timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
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

#[derive(Debug, Copy, Clone)]
pub enum BoostStrategy {
    Incentive, // Everyone who votes is eligible, regardless of choice
    Bribe(u8), // Only those who voted for the specific choice are eligible
}

fn validate_choice(choice: u8, boost_strategy: BoostStrategy) -> Result<(), ServerError> {
    match boost_strategy {
        BoostStrategy::Incentive => Ok(()),
        BoostStrategy::Bribe(boosted_choice) => {
            if choice != boosted_choice {
                Err(ServerError::ErrorString(
                    "voter is not eligible: choice is not boosted".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
struct Vote {
    voting_power: f64,
    choice: u8,
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

    let res = client.post(HUB_URL).json(&request_body).send().await?;
    let response_body: GraphQLResponse<votes_query::ResponseData> = res.json().await?;
    let votes = response_body
        .data
        .ok_or("missing data from the hub")?
        .votes
        .ok_or("missing votes fomr the hub")?;

    let vote = votes
        .into_iter()
        .next()
        .ok_or("missing vote from the hub")?
        .ok_or("missing first vote from the hub?")?;

    Ok(Vote {
        voting_power: vote.vp.ok_or("missing vp from the hub")?,
        choice: vote.choice,
    })
}

#[derive(Debug)]
struct Proposal {
    type_: String,
    score: u128,
    end: u64,
}

impl TryFrom<proposal_query::ProposalQueryProposal> for Proposal {
    type Error = ServerError;

    fn try_from(proposal: proposal_query::ProposalQueryProposal) -> Result<Self, Self::Error> {
        let proposal_type = proposal.type_.ok_or("missing proposal type from the hub")?;
        let proposal_score = proposal
            .scores_total
            .ok_or("missing proposal score from the hub")? as u128;
        let proposal_end = proposal.end.try_into()?;

        Ok(Proposal {
            type_: proposal_type,
            score: proposal_score,
            end: proposal_end,
        })
    }
}

async fn get_proposal_info(
    client: &reqwest::Client,
    proposal_id: &str,
) -> Result<Proposal, ServerError> {
    let variables = proposal_query::Variables {
        id: proposal_id.to_owned(),
    };

    let request_body = ProposalQuery::build_query(variables);

    let res = client.post(HUB_URL).json(&request_body).send().await?;
    let response_body: GraphQLResponse<proposal_query::ResponseData> = res.json().await?;
    let proposal_query: proposal_query::ProposalQueryProposal = response_body
        .data
        .ok_or("missing data from the hub")?
        .proposal
        .ok_or("missing proposal data from the hub")?;
    Proposal::try_from(proposal_query)
}

pub async fn health_handler() -> Result<impl IntoResponse, ServerError> {
    Ok(axum::response::Html("Healthy!"))
}

pub async fn forward_handler(
    Extension(state): Extension<State>,
    Json(p): Json<Value>,
) -> Result<impl IntoResponse, ServerError> {
    println!("1");
    let params: ForwardParams = serde_json::from_value(p)?;

    println!("2");
    let res = state.client.get(params.to).send().await?;
    println!("3");
    let b = res.text().await.unwrap();
    Ok(axum::response::Html(b))
}

// TODO: add signature testing
