use axum::routing::{get, post};
use axum::{Extension, Router};
use boost_guard::routes::{handle_create_vouchers, handle_get_rewards, handle_health};
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
        .route("/create-vouchers", post(handle_create_vouchers))
        .route("/get-rewards", post(handle_get_rewards))
        .route(
            "/get-lottery-winners",
            post(boost_guard::routes::handle_get_lottery_winners),
        )
        .route("/health", get(handle_health))
        .layer(Extension(state))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http;
    use boost_guard::routes::{
        CreateVouchersResponse, GetLotteryWinnerQueryParams, GetLotteryWinnersResponse,
        GetRewardsResponse, QueryParams,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    // TODO: those test use fixed proposals and voter addresses, but these change from time to time as we delete
    // proposals from the hub... we should probably settle for a fixed proposal and voter address and use those
    #[tokio::test]
    async fn test_create_vouchers() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0x5c4d271f77150458cb0265cd5b473dd970bb5fa1fe4e006775c52b94c8e363a1"
                .to_string(),
            voter_address: "0xc83A9e69012312513328992d454290be85e95101".to_string(),
            boosts: vec![("0".to_string(), "1".to_string())],
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
            proposal_id: "0x5c4d271f77150458cb0265cd5b473dd970bb5fa1fe4e006775c52b94c8e363a1"
                .to_string(),
            voter_address: "0xc83A9e69012312513328992d454290be85e95101".to_string(),
            boosts: vec![("0".to_string(), "1".to_string())],
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
    async fn test_get_lottery_winners() {
        let app = super::app();
        let query = GetLotteryWinnerQueryParams {
            proposal_id: "0xe2f9d5694f6af28b61b357b80752566654b026343dbdf09d36d291b6325dedb3"
                .to_string(),
            boost_id: "12".to_string(),
            chain_id: "11155111".to_string(),
        };

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .uri("/get-lottery-winners")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&query).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let response: Result<GetLotteryWinnersResponse, _> = serde_json::from_slice(&bytes);
        if response.is_err() {
            println!("ERROR: {}", String::from_utf8(bytes.to_vec()).unwrap());
            panic!("failed test");
        } else {
            println!("OK: {:?}", response.unwrap());
        }
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
