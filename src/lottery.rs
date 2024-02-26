use crate::routes::{every_vote_query, BoostInfo, EveryVoteQuery, ProposalInfo, VoteInfo};
use crate::{ServerError, BEACONCHAIN_API_KEY};
use crate::{EPOCH_URL, HUB_URL, SLOT_URL};
use cached::proc_macro::cached;
use cached::TimedSizedCache;
use durations::WEEK;
use ethers::types::{Address, U256};
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
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
    let seed = get_randao_reveal(proposal_info.end).await?;

    // Every voter is eligible to the same reward!
    if votes.len() <= num_winners as usize {
        return Ok(votes.into_iter().map(|v| (v.voter, prize)).collect());
    }

    Ok(draw_winners(votes, seed, num_winners, prize))
}

fn draw_winners(
    votes: Vec<VoteInfo>,
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

    println!("nearest slot: {}", nearest_slot);
    // Step 2
    let slot_url = format!(
        "{}{}?apikey={}",
        SLOT_URL.as_str(),
        nearest_slot,
        BEACONCHAIN_API_KEY.as_str()
    );
    println!("slot url: {}", slot_url);
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

    println!("epoch details: {:?}", epoch_details["data"]);
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
    let hash_result = hasher.finalize();

    // Step 3: Convert the hash bytes to a fixed-size array for the seed
    let seed = hash_result
        .try_into()
        .expect("Hash output size is incorrect for seed");

    Ok(seed)
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
