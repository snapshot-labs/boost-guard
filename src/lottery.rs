use crate::routes::{every_vote_query, BoostInfo, EveryVoteQuery, ProposalInfo, VoteInfo};
use crate::ServerError;
use crate::HUB_URL;
use cached::proc_macro::cached;
use cached::TimedSizedCache;
use durations::WEEK;
use ethers::types::{Address, U256};
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::collections::HashMap;
use std::str::FromStr;

// LRU cache that uses `boost_id` and `chain_id` as keys
#[cached(
    result = true,
    sync_writes = true,
    type = "TimedSizedCache<String, HashMap<Address, U256>>",
    create = "{ TimedSizedCache::with_size_and_lifespan(500, 3 * WEEK.as_secs())}",
    convert = r#"{ format!("{}{}", boost_info.id, boost_info.chain_id) }"#
)]
pub async fn cached_lottery_winners(
    client: Option<&reqwest::Client>,
    boost_info: &BoostInfo,
    proposal_info: &ProposalInfo,
    num_winners: u32,
) -> Result<HashMap<Address, U256>, ServerError> {
    let variables = every_vote_query::Variables {
        proposal: proposal_info.id.to_owned(),
    };
    let request_body = EveryVoteQuery::build_query(variables);
    let query_results: GraphQLResponse<every_vote_query::ResponseData> = client
        .expect("client should be here")
        .post(HUB_URL.as_str())
        .json(&request_body)
        .send()
        .await?
        .json()
        .await?;

    let mut votes = query_results
        .data
        .ok_or("lottery: missing data from the hub")?
        .votes
        .ok_or("lottery: missing votes from the hub")?
        .into_iter()
        .map(|v| {
            let vote_data = v.ok_or("lottery: missing vote info from the hub")?;
            let vote = VoteInfo {
                voter: Address::from_str(&vote_data.voter)
                    .map_err(|e| format!("lottery: {:?}", e))?,
                voting_power: vote_data.vp.ok_or("missing vp from the hub")?,
                choice: vote_data.choice,
            };
            Ok::<VoteInfo, ServerError>(vote)
        })
        .collect::<Result<Vec<VoteInfo>, _>>()?;

    if let Some(boosted_choice) = boost_info.params.eligibility.boosted_choice() {
        votes.retain(|v| v.choice == boosted_choice);
    }

    let prize = boost_info.pool_size / num_winners;
    let seed = ChaCha20Rng::from_entropy().gen(); // todo: e.g from block ranDAO reveal

    // Every voter is eligible to the same reward!
    if votes.len() <= num_winners as usize {
        let prize = boost_info.pool_size / votes.len() as u32;
        return Ok(votes.into_iter().map(|v| (v.voter, prize)).collect());
    }

    Ok(draw_winners(votes, seed, num_winners, prize))
}

fn draw_winners(
    votes: Vec<VoteInfo>,
    seed: u64,
    num_winners: u32,
    prize: U256,
) -> HashMap<Address, U256> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut winners = HashMap::with_capacity(num_winners as usize);

    let mut set = std::collections::HashSet::new();

    // Construct the cumulative weights (e.g; [1, 2, 3, 4] -> [1, 3, 6, 10])
    let mut cumulative_weights = Vec::with_capacity(votes.len());
    let mut curr = votes[0].voting_power;
    cumulative_weights.push(curr);
    for v in votes.iter().skip(1) {
        curr += v.voting_power;
        cumulative_weights.push(curr);
    }

    // TODO: we could optimize by sorting by votes and then poping the last element (which has the highest
    // probability of getting picked). For now, we don't optimize.
    let range = 0.0..*cumulative_weights.last().unwrap();
    for _ in 0..num_winners {
        let winner = loop {
            // Generate a random number between 0 and the highest element of the cumulative weights
            let rnd: f64 = rng.gen_range(range.clone());
            // Get the index of the first element that is greater than or equal to the random number
            let idx = cumulative_weights.iter().position(|x| *x >= rnd).unwrap();
            // Get the corresponding winner address
            let winner = votes.get(idx).unwrap().voter;

            // If the winner has been selected before, draw again.
            if set.contains(&winner) {
                continue;
            } else {
                break winner;
            }
        };

        // Add winner to the set
        set.insert(winner);
        // Add winner to the winners map
        winners.insert(winner, prize);
    }
    winners
}

#[cfg(test)]
mod test_draw_winners {
    use super::draw_winners;
    use super::VoteInfo;
    use super::U256;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_randomness() {
        let vote1 = VoteInfo {
            voting_power: 99.0,
            ..Default::default()
        };
        let vote2 = VoteInfo {
            voting_power: 1.0,
            ..Default::default()
        };
        let votes = vec![vote1.clone(), vote2.clone()];
        println!("{}, {}", vote1.voter, vote2.voter);
        let prize = U256::from(10);

        let mut rng = ChaCha8Rng::from_entropy();
        let mut num = 0;

        // Draw 10000 times, expect voter 2 to get picked about 100 times.
        for _ in 0..10000 {
            let winners = draw_winners(votes.clone(), rng.gen(), 1, prize);
            if winners.get(&vote2.voter).is_some() {
                num += 1;
            }
        }

        // Allow for a margin of error
        assert!(num >= 70);
        assert!(num <= 130);
    }

    #[test]
    fn select_two() {
        let vote1 = VoteInfo {
            voting_power: 98.0,
            ..Default::default()
        };
        let vote2 = VoteInfo {
            voting_power: 1.0,
            ..Default::default()
        };
        let vote3 = VoteInfo {
            voting_power: 1.0,
            ..Default::default()
        };
        let votes = vec![vote1.clone(), vote2.clone(), vote3.clone()];
        println!("{}, {}, {}", vote1.voter, vote2.voter, vote3.voter);
        let prize = U256::from(10);

        let mut rng = ChaCha8Rng::from_entropy();

        let winners = draw_winners(votes, rng.gen(), 2, prize);
        assert_eq!(winners.len(), 2);
    }

    #[test]
    fn test_speed() {
        let votes = (0..1000000)
            .enumerate()
            .map(|(i, _)| VoteInfo {
                voting_power: i as f64,
                ..Default::default()
            })
            .collect();
        let prize = U256::from(10);

        let mut rng = ChaCha8Rng::from_entropy();

        let start = std::time::Instant::now();
        println!("start");
        let _ = draw_winners(votes, rng.gen(), 1000, prize);
        let finish = std::time::Instant::now();
        println!("Time: {:?}", finish - start);
    }
}
