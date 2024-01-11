use axum::response::{IntoResponse, Response};
use hyper::http::StatusCode;

pub mod routes;
pub mod signatures;

use std::env;
extern crate dotenv;
use dotenv::dotenv;
#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref HUB_URL: String = {
        dotenv().ok();
        env::var("HUB_URL").expect("Please add HUB_URL to your environment or .env file.")
    };
    static ref SUBGRAPH_URL: String = env::var("SUBGRAPH_URL")
        .expect("Please add SUBGRAPH_URL to your environment or .env file.");
}

pub enum ServerError {
    ErrorString(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let ServerError::ErrorString(body) = self;

        // its often easiest to implement `IntoResponse` by calling other implementations
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl<T: std::fmt::Debug> From<T> for ServerError {
    fn from(error: T) -> Self {
        ServerError::ErrorString(format!("{:?}", error))
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub client: reqwest::Client,
    pub wallet: ethers::signers::LocalWallet,
}
