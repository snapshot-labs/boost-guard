use crate::routes::{BoostInfo, ProposalInfo, Vote};
use crate::{ServerError, BEACONCHAIN_API_KEY};
use crate::{EPOCH_URL, MYRIAD, SLOT_URL};
use cached::proc_macro::cached;
use cached::TimedSizedCache;
use durations::WEEK;
use ethers::types::{Address, U256};
use mysql_async::prelude::Queryable;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str::FromStr;

const FIRST_MERGED_SLOT: u64 = 4700013;
const FIRST_MERGED_SLOT_TIMESTAMP: u64 = 1663224179;

// LRU cache that uses `boost_id` and `chain_id` as keys
#[cached(
    result = true,
    sync_writes = true,
    type = "TimedSizedCache<String, HashMap<Address, U256>>",
    create = "{ TimedSizedCache::with_size_and_lifespan(500, 3 * WEEK.as_secs())}",
    convert = r#"{ format!("{}{}", boost_info.id, boost_info.chain_id) }"#
)]
pub async fn cached_lottery_winners(
    pool: &mysql_async::Pool,
    boost_info: &BoostInfo,
    proposal_info: &ProposalInfo,
    num_winners: u32,
    limit: Option<u16>,
) -> Result<HashMap<Address, U256>, ServerError> {
    let choice_constraint =
        if let Some(boosted_choice) = boost_info.params.eligibility.boosted_choice() {
            format!("AND choice = {}", boosted_choice)
        } else {
            "".to_string()
        };

    let query = format!(
        "SELECT voter, vp, choice
        FROM votes
        WHERE proposal = '{}'
        {}
        ORDER BY vp DESC;",
        proposal_info.id, choice_constraint
    );

    let mut conn = pool.get_conn().await?;
    let result: Vec<(String, f64, u32)> = conn.query(query).await?;
    conn.disconnect().await?;
    
    if result.is_empty() {
        return Err("no eligibles votes found")?;
    }

    let mut votes = result
        .into_iter()
        .map(|(voter, vp, _)| {
            Ok(Vote {
                voter: Address::from_str(&voter)?,
                voting_power: vp,
            })
        })
        .collect::<Result<Vec<Vote>, ServerError>>()?;

    // If there are not enough voters, then every voter is eligible to the same reward
    if votes.len() <= num_winners as usize {
        let prize = boost_info.pool_size / votes.len() as u32;
        return Ok(votes.into_iter().map(|v| (v.voter, prize)).collect());
    }

    if let Some(limit) = limit {
        adjust_vote_weights(&mut votes, boost_info.decimals, proposal_info.score, limit)?;
    }

    let prize = boost_info.pool_size / num_winners;
    let seed = get_randao_reveal(proposal_info.end).await?;

    Ok(draw_winners(votes, seed, num_winners, prize))
}

// Adjust the voting power of the voters to respect the limit.
// The limit is given in base `10_000`, meaning a limit of `1000` means "no one should have more than 10% chances of getting picked".
// To enforce that, we iterate through the voters' voting power and adjust their voting power to respect the limit.
// If there are not enough voters to reach the limit, the limit will be ignored (e.g: if the limit is 1%, and there is only one voter,
// the voter will get 100% of the prize). Limit will be ignored if set to 0.
// The array of `votes` is assumed to be sorted by voting power.
fn adjust_vote_weights(
    votes: &mut [Vote],
    decimals: u8,
    score: f64,
    limit: u16,
) -> Result<(), ServerError> {
    if limit == 0 {
        // log needed
        return Ok(());
    }

    if limit == MYRIAD {
        // log needed
        return Ok(());
    }

    // Ensure the vector is sorted
    if votes
        .windows(2)
        .any(|w| w[0].voting_power < w[1].voting_power)
    {
        return Err(ServerError::ErrorString("votes are not sorted".to_string()));
    }

    if votes.len() < (MYRIAD as f64 / limit as f64).ceil() as usize {
        // log needed "not enough voters to enforce the limit"
        return Ok(());
    }

    let pow = 10_f64.powi(decimals as i32);

    // The "voting power" remaining. At each iteration, we will subtract the voting power of the voter.
    let mut remaining_score = U256::from((score * pow) as u128);

    // The "adjust voting power" remaining. At each iteration, we will subtract the adjusted voting power of the voter.
    let mut adjusted_remaining_score = remaining_score;

    // The maximum adjusted voting power that can be assigned to any voter
    let vp_limit = remaining_score * limit / MYRIAD;

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
    votes: Vec<Vote>,
    seed: [u8; 32],
    num_winners: u32,
    prize: U256,
) -> HashMap<Address, U256> {
    let mut rng = ChaCha20Rng::from_seed(seed);
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

// This function tries to map a timestamp to a randao reveal.
// A randao reveal is a source of randomness provided by the beacon chain. Each epoch has a randao reveal.
// This function tries to find the epoch that contains the timestamp and returns the randao reveal of that epoch.
// It does so by following these steps:
// Step 1: Find the closest slot corresponding to the timestamp (round up to nearest multiple of 12, because slots are 12 seconds long).
// Step 2: Query the SLOT_URL to get the corresponding slot, and extract its epoch.
// Step 3: Query the EPOCH_URL to get the corresponding epoch.
// Step 4: Ensure the epoch is finalized, and return its randao reveal.
async fn randao_from_timestamp(timestamp: u64) -> Result<String, ServerError> {
    let client = reqwest::Client::new();

    // Step 1
    let elapsed = timestamp - FIRST_MERGED_SLOT_TIMESTAMP;
    let rounded_elapsed = elapsed + (12 - elapsed % 12);
    let elapsed_slots = rounded_elapsed / 12;
    let nearest_slot = FIRST_MERGED_SLOT + elapsed_slots;

    // Step 2
    let slot_url = format!(
        "{}{}?apikey={}",
        SLOT_URL.as_str(),
        nearest_slot,
        BEACONCHAIN_API_KEY.as_str()
    );
    let slot: Value = client.get(&slot_url).send().await?.json().await?;
    let epoch = slot["data"]["epoch"]
        .as_u64()
        .ok_or("failed to parse epoch")?;

    // Step 3
    let epoch_url = format!(
        "{}{}?apikey={}",
        EPOCH_URL.as_str(),
        epoch,
        BEACONCHAIN_API_KEY.as_str()
    );
    let epoch_details: Value = client.get(&epoch_url).send().await?.json().await?;

    // Step 4
    let finalized = epoch_details["data"]["finalized"]
        .as_bool()
        .ok_or("finalized is not a boolean")?;
    if !finalized {
        return Err("epoch is not finalized".into());
    }

    let randao_reveal = slot["data"]["randaoreveal"]
        .as_str()
        .ok_or("randao_reveal is not a string")?
        .to_string();

    Ok(randao_reveal)
}

// Create a 32bytes seed with the sha256 hash of the randao reveal corresponding to
// the next nearest epoch of the given timestamp.
async fn get_randao_reveal(timestamp: u64) -> Result<[u8; 32], ServerError> {
    // Step 1: Get the randao reveal from the chain
    let randao = randao_from_timestamp(timestamp).await?;
    let bytes = hex::decode(&randao[2..]).unwrap();

    // Step 2: Hash the byte array
    let mut hasher = Sha256::new();
    hasher.update(&bytes);

    // Step 3: Convert the hash bytes to a fixed-size array for the seed
    let seed = hasher.finalize().into();

    Ok(seed)
}

#[cfg(test)]
mod test_draw_winners {
    use super::draw_winners;
    use super::Vote;
    use super::U256;
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_randomness() {
        let vote1 = Vote {
            voting_power: 99.0,
            ..Default::default()
        };
        let vote2 = Vote {
            voting_power: 1.0,
            ..Default::default()
        };
        let votes = vec![vote1.clone(), vote2.clone()];
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
        let vote1 = Vote {
            voting_power: 98.0,
            ..Default::default()
        };
        let vote2 = Vote {
            voting_power: 1.0,
            ..Default::default()
        };
        let vote3 = Vote {
            voting_power: 1.0,
            ..Default::default()
        };
        let votes = vec![vote1.clone(), vote2.clone(), vote3.clone()];
        let prize = U256::from(10);

        let mut rng = ChaCha8Rng::from_entropy();

        let winners = draw_winners(votes, rng.gen(), 2, prize);
        assert_eq!(winners.len(), 2);
    }

    #[test]
    #[cfg(feature = "expensive_tests")]
    fn test_speed() {
        let votes = (0..1000000)
            .enumerate()
            .map(|(i, _)| Vote {
                voting_power: i as f64,
                ..Default::default()
            })
            .collect();
        let prize = U256::from(10);

        let mut rng = ChaCha8Rng::from_entropy();

        let start = std::time::Instant::now();
        let _ = draw_winners(votes, rng.gen(), 1000, prize);
        let finish = std::time::Instant::now();
        println!("Time: {:?}", finish - start);
    }
}

#[cfg(test)]
mod test_adjust_vote_weights {
    use super::adjust_vote_weights;
    use super::Vote;

    #[test]
    fn test_adjust_vote_weights_half() {
        let mut votes = vec![
            Vote {
                voting_power: 900.0,
                ..Default::default()
            },
            Vote {
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
            Vote {
                voting_power: 900.0,
                ..Default::default()
            },
            Vote {
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
            Vote {
                voting_power: 900.0,
                ..Default::default()
            },
            Vote {
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
    fn test_adjust_limit_fourty() {
        let mut votes = vec![
            Vote {
                voting_power: 10.0,
                ..Default::default()
            },
            Vote {
                voting_power: 10.0,
                ..Default::default()
            },
            Vote {
                voting_power: 1.0,
                ..Default::default()
            },
            Vote {
                voting_power: 1.0,
                ..Default::default()
            },
        ];
        let decimals = 18;
        let score = votes.iter().map(|v| v.voting_power).sum::<f64>();
        let limit = 4000; // 40 %

        adjust_vote_weights(&mut votes, decimals, score, limit).unwrap();

        assert_eq!(votes[0].voting_power, 8.8);
        assert_eq!(votes[1].voting_power, 8.8);
        assert_eq!(votes[2].voting_power, 2.2);
        assert_eq!(votes[3].voting_power, 2.2);
    }

    #[test]
    fn test_adjust_limit_no_op_rounded() {
        let mut votes = vec![
            Vote {
                voting_power: 900.0,
                ..Default::default()
            },
            Vote {
                voting_power: 50.0,
                ..Default::default()
            },
            Vote {
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
            Vote {
                voting_power: 800.0,
                ..Default::default()
            },
            Vote {
                voting_power: 100.0,
                ..Default::default()
            },
            Vote {
                voting_power: 50.0,
                ..Default::default()
            },
            Vote {
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
            Vote {
                voting_power: 458.0,
                ..Default::default()
            },
            Vote {
                voting_power: 200.0,
                ..Default::default()
            },
            Vote {
                voting_power: 180.0,
                ..Default::default()
            },
            Vote {
                voting_power: 150.0,
                ..Default::default()
            },
            Vote {
                voting_power: 5.0,
                ..Default::default()
            },
            Vote {
                voting_power: 4.0,
                ..Default::default()
            },
            Vote {
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
