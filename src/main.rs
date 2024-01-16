use axum::routing::{get, post};
use axum::{Extension, Router};
use boost_guard::routes::{create_vouchers_handler, get_rewards_handler, health_handler};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpListener;
extern crate dotenv;

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let port: u16 = env::var("PORT")
        .map(|val| val.parse::<u16>().unwrap())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    dotenv().ok();

    let client = reqwest::Client::new();
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = ethers::signers::LocalWallet::from_str(&private_key)
        .expect("failed to create a local wallet");
    let state = boost_guard::State { client, wallet };

    Router::new()
        .route("/create-vouchers", post(create_vouchers_handler))
        .route("/get-rewards", post(get_rewards_handler))
        .route("/health", get(health_handler))
        .layer(Extension(state))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http;
    use boost_guard::routes::{CreateVouchersResponse, GetRewardsResponse, QueryParams};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    // TODO: those test use fixed proposals and voter addresses, but these change from time to time as we delete
    // proposals from the hub... we should probably settle for a fixed proposal and voter address and use those
    #[tokio::test]
    async fn test_create_vouchers() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0xc14e29f756952d92a42814a51aa60717a10297a3f92f0d20f0e90bb2bb6c606b"
                .to_string(),
            voter_address: "0x3901D0fDe202aF1427216b79f5243f8A022d68cf".to_string(),
            boosts: vec![("23".to_string(), "11155111".to_string())],
        };

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .uri("/create-vouchers")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&query).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let response: Result<Vec<CreateVouchersResponse>, _> = serde_json::from_slice(&bytes);
        if response.is_err() {
            println!("ERROR: {}", String::from_utf8(bytes.to_vec()).unwrap());
            panic!();
        } else {
            println!("OK: {:?}", response.unwrap());
        }
    }

    #[tokio::test]
    async fn test_get_rewards() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0xc14e29f756952d92a42814a51aa60717a10297a3f92f0d20f0e90bb2bb6c606b"
                .to_string(),
            voter_address: "0x3901D0fDe202aF1427216b79f5243f8A022d68cf".to_string(),
            boosts: vec![("23".to_string(), "11155111".to_string())],
        };

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .uri("/get-rewards")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&query).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let response: Result<Vec<GetRewardsResponse>, _> = serde_json::from_slice(&bytes);
        if response.is_err() {
            println!("ERROR: {}", String::from_utf8(bytes.to_vec()).unwrap());
            panic!();
        } else {
            println!("OK: {:?}", response.unwrap());
        }
    }

    #[tokio::test]
    async fn test_invalid_proposal_type() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0x2f488ec3a0b9b5d731812395f2aa99718df7d380b6c6c0539fec16ae53b3e1fc"
                .to_string(),
            voter_address: "voter_address".to_string(),
            boosts: vec![("0x1234".to_string(), "0x42".to_string())],
        };

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .uri("/create-vouchers")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&[&query, &query]).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status() == http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = super::app();
        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status() == http::StatusCode::OK);
    }
}
