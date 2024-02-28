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
    limit: Option<u16>,
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

    // Every voter is eligible to the same reward!
    if votes.len() <= num_winners as usize {
        let prize = boost_info.pool_size / votes.len() as u32;
        return Ok(votes.into_iter().map(|v| (v.voter, prize)).collect());
    }

    if let Some(limit) = limit {
        adjust_vote_weights(&mut votes, boost_info.decimals, proposal_info.score, limit)?;
    }

    let prize = boost_info.pool_size / num_winners;
    let seed = ChaCha20Rng::from_entropy().gen(); // todo: e.g from block ranDAO reveal

    Ok(draw_winners(votes, seed, num_winners, prize))
}

// Adjust the voting power of the voters to respect the limit.
// The limit is given in base `10_000`, meaning a limit of `1000` means "no one should have more than 10% chances of getting picked".
// To enforce that, we iterate through the voters' voting power and adjust their voting power to respect the limit.
// If there are not enough voters to reach the limit, the limit will be ignored (e.g: if the limit is 1%, and there is only one voter,
// the voter will get 100% of the prize). Limit will be ignored if set to 0.
// The array of `votes` is assumed to be sorted by voting power.
fn adjust_vote_weights(
    votes: &mut [VoteInfo],
    decimals: u8,
    score: f64,
    limit: u16,
) -> Result<(), ServerError> {
    // Ensure the vector is sorted
    if votes
        .windows(2)
        .any(|w| w[0].voting_power < w[1].voting_power)
    {
        return Err(ServerError::ErrorString("votes are not sorted".to_string()));
    }

    if limit == 0 {
        // log needed
        return Ok(());
    }

    if votes.len() < (10_000.0 / limit as f64).ceil() as usize {
        // log needed "not enough voters to enforce the limit"
        return Ok(());
    }

    let pow = 10_f64.powi(decimals as i32);

    // The "voting power" remaining. At each iteration, we will subtract the voting power of the voter.
    let mut remaining_score = U256::from((score * pow) as u128);

    // The "adjust voting power" remaining. At each iteration, we will subtract the adjusted voting power of the voter.
    let mut adjusted_remaining_score = remaining_score;

    // The maximum adjusted voting power that can be assigned to any voter
    let vp_limit = remaining_score * limit / 10_000; // TODO: constant

    votes.iter_mut().for_each(|v| {
        let vp = U256::from((v.voting_power * pow) as u128);
        // If the user reaches the limit, assign the limit, else assign the correct ratio.
        let adjusted_voting_power =
            std::cmp::min(vp_limit, adjusted_remaining_score * vp / remaining_score);

        // Subtract the voting power
        remaining_score -= vp;
        // Subtract the adjusted voting power
        adjusted_remaining_score -= adjusted_voting_power;

        // Update the voter's voting power
        v.voting_power = adjusted_voting_power.as_u128() as f64 / pow;
    });

    Ok(())
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

#[cfg(test)]
mod test_adjust_vote_weights {
    use super::adjust_vote_weights;
    use super::VoteInfo;

    #[test]
    fn test_adjust_vote_weights_half() {
        let mut votes = vec![
            VoteInfo {
                voting_power: 900.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 100.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 5000; // 50 %

        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 500.0);
        assert_eq!(votes[1].voting_power, 500.0);
    }

    #[test]
    fn test_adjust_no_op() {
        let mut votes = vec![
            VoteInfo {
                voting_power: 900.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 100.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 100; // 1 %

        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 900.0);
        assert_eq!(votes[1].voting_power, 100.0);
    }

    #[test]
    fn test_adjust_limit_zero() {
        let mut votes = vec![
            VoteInfo {
                voting_power: 900.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 100.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 0; // 0 %

        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 900.0);
        assert_eq!(votes[1].voting_power, 100.0);
    }

    #[test]
    fn test_adjust_limit_no_op_rounded() {
        let mut votes = vec![
            VoteInfo {
                voting_power: 900.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 50.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 50.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 3000; // 30 %

        // Would need 4 voters but we only have three so no-op
        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 900.0);
        assert_eq!(votes[1].voting_power, 50.0);
        assert_eq!(votes[2].voting_power, 50.0);
    }

    #[test]
    fn test_adjust_limit_rounded() {
        let mut votes = vec![
            VoteInfo {
                voting_power: 800.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 100.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 50.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 50.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 3000; // 30 %

        // We indeed have 4 voters, votes should get adjusted
        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 300.0);
        assert_eq!(votes[1].voting_power, 300.0);
        assert_eq!(votes[2].voting_power, 200.0);
        assert_eq!(votes[2].voting_power, 200.0);
    }

    #[test]
    fn test_adjust_vote_weights() {
        let mut votes = vec![
            VoteInfo {
                voting_power: 458.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 200.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 180.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 150.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 5.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 4.0,
                ..Default::default()
            },
            VoteInfo {
                voting_power: 3.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 2000; // 20 %

        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 200.0);
        assert_eq!(votes[1].voting_power, 200.0);
        assert_eq!(votes[2].voting_power, 200.0);
        assert_eq!(votes[3].voting_power, 200.0);
        assert_eq!(votes[4].voting_power, 83.33333333333333);
        assert_eq!(votes[5].voting_power, 66.66666666666666);
        assert_eq!(votes[6].voting_power, 50.0);
    }
}
