use axum::response::{IntoResponse, Response};
use ethers::types::Address;
use hyper::http::StatusCode;
use std::collections::HashSet;

pub mod lottery;
pub mod routes;
pub mod signatures;
pub mod tokens;

use std::env;
extern crate dotenv;
use dotenv::dotenv;
use std::collections::HashMap;

#[macro_use]
extern crate lazy_static;

pub const MYRIAD: u16 = 10_000;

lazy_static! {
    static ref HUB_URL: String = {
        dotenv().ok();

        return env::var("HUB_URL").expect("Please add HUB_URL to your environment or .env file.");
    };
    static ref SUBGRAPH_URLS: HashMap<&'static str, String> = {
        dotenv().ok();
        let mut map = HashMap::new();

        map.insert(
            "1",
            env::var("MAINNET_SUBGRAPH_URL")
                .expect("Please add MAINNET_SUBGRAPH_URL to your environment or .env file."),
        );

        map.insert(
            "11155111",
            env::var("SEPOLIA_SUBGRAPH_URL")
                .expect("Please add SEPOLIA_SUBGRAPH_URL to your environment or .env file."),
        );

        map
    };
    static ref BEACONCHAIN_API_KEY: String = env::var("BEACONCHAIN_API_KEY")
        .expect("Please add BEACONCHAIN_API_KEY to your environment or .env file");
    static ref BOOST_NAME: String =
        env::var("BOOST_NAME").expect("Please add BOOST_NAME to your environment or .env file");
    static ref BOOST_VERSION: String = env::var("BOOST_VERSION")
        .expect("Please add BOOST_VERSION to your environment or .env file");
    static ref VERIFYING_CONTRACT: String = env::var("VERIFYING_CONTRACT")
        .expect("Please add VERIFYING_CONTRACT to your environment or .env file");
    static ref EPOCH_URL: String =
        env::var("EPOCH_URL").expect("Please add EPOCH_URL to your environment or .env file");
    static ref SLOT_URL: String =
        env::var("SLOT_URL").expect("Please add SLOT_URL to your environment or .env file");
    static ref DISABLED_TOKENS: HashSet<(Address, &'static str)> =
        tokens::create_disabled_token_list();
}

#[derive(Debug, PartialEq, Clone)]
pub enum ServerError {
    ErrorString(String),
    ProposalStillInProgress,
}

impl<T: std::string::ToString + Sized> From<T> for ServerError {
    fn from(err: T) -> Self {
        ServerError::ErrorString(err.to_string())
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        match self {
            ServerError::ErrorString(body) => {
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            ServerError::ProposalStillInProgress => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Proposal has not ended yet",
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub client: reqwest::Client,
    pub pool: mysql_async::Pool,
    pub wallet: ethers::signers::LocalWallet,
}
