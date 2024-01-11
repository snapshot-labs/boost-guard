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
        .expect("failed to create a local wallet"); // todo check hex
    let state = boost_guard::State { client, wallet };

    Router::new()
        .route("/create-vouchers", post(create_vouchers_handler)) // todo: create-voucherS
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

    #[tokio::test]
    async fn test_create_vouchers() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0x04c7512be9a0a0d8c12885a9c6cf84354749dc1e4cdd4eda64b4cb33bc126880"
                .to_string(),
            voter_address: "0xc83A9e69012312513328992d454290be85e95101".to_string(),
            boosts: vec![
                ("16".to_string(), "11155111".to_string()),
                ("17".to_string(), "11155111".to_string()),
                ("18".to_string(), "11155111".to_string()),
                ("19".to_string(), "11155111".to_string())
                ],
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
            proposal_id: "0x04c7512be9a0a0d8c12885a9c6cf84354749dc1e4cdd4eda64b4cb33bc126880"
                .to_string(),
            voter_address: "0xc83A9e69012312513328992d454290be85e95101".to_string(),
            boosts: vec![
                ("16".to_string(), "11155111".to_string()),
                ("17".to_string(), "11155111".to_string()),
                ("18".to_string(), "11155111".to_string()),
                ("19".to_string(), "11155111".to_string())
                ],
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
