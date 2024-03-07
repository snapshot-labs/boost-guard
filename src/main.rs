use axum::routing::{get, post};
use axum::{Extension, Router};
use boost_guard::routes::{handle_create_vouchers, handle_get_rewards, handle_health, handle_root};
use mysql_async::Pool;
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

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    let _ = tracing::subscriber::set_default(subscriber);

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = Pool::new(database_url.as_str());

    let client = reqwest::Client::new();

    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let wallet = ethers::signers::LocalWallet::from_str(&private_key)
        .expect("failed to create a local wallet");
    let state = boost_guard::State {
        client,
        pool,
        wallet,
    };

    Router::new()
        .route("/create-vouchers", post(handle_create_vouchers))
        .route("/get-rewards", post(handle_get_rewards))
        .route(
            "/get-lottery-winners",
            post(boost_guard::routes::handle_get_lottery_winners),
        )
        .route("/health", get(handle_health))
        .route("/", get(handle_root))
        .layer(Extension(state))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http;
    use boost_guard::routes::{
        CreateVouchersResponse, GetLotteryWinnerQueryParams, GetLotteryWinnersResponse,
        GetRewardsResponse, GuardInfoResponse, QueryParams,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    const WINNER: &str = "0x3901d0fde202af1427216b79f5243f8a022d68cf";
    const PROPOSAL_ID: &str = "0xc3beb923ad594240e964324c07b6ed0828687d149c3ef30085e8ca844cf11ee1";
    const BOOST_ID: &str = "3";
    const CHAIN_ID: &str = "11155111";

    #[tokio::test]
    async fn test_create_vouchers() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: PROPOSAL_ID.to_string(),
            voter_address: WINNER.to_string(),
            boosts: vec![(BOOST_ID.to_string(), CHAIN_ID.to_string())],
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
            let result = response.unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].signature, "0x3099eca443b11fbcc85e0e5a772eb0276aceb2060d440edce2474b8bb5e28ce0727180bf08b88030bb0d5ed7592dd36b2c42622777cb485cfa47baae321772eb1c");
            assert_eq!(result[0].reward, "10000000000000000");
            assert_eq!(result[0].chain_id, CHAIN_ID);
            assert_eq!(result[0].boost_id, BOOST_ID);
        }
    }

    #[tokio::test]
    async fn test_get_rewards() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: PROPOSAL_ID.to_string(),
            voter_address: WINNER.to_string(),
            boosts: vec![(BOOST_ID.to_string(), CHAIN_ID.to_string())],
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
            let result = response.unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].reward, "10000000000000000");
            assert_eq!(result[0].chain_id, CHAIN_ID);
            assert_eq!(result[0].boost_id, BOOST_ID);
        }
    }

    #[tokio::test]
    async fn test_get_lottery_winners() {
        let app = super::app();
        let query = GetLotteryWinnerQueryParams {
            proposal_id: PROPOSAL_ID.to_string(),
            boost_id: BOOST_ID.to_string(),
            chain_id: CHAIN_ID.to_string(),
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
            let result = response.unwrap();
            assert_eq!(result.winners.len(), 1);
            assert_eq!(result.winners[0], WINNER);
            assert_eq!(result.prize, "10000000000000000");
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

    #[tokio::test]
    async fn test_root() {
        let app = super::app();
        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::GET)
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status() == http::StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let response: GuardInfoResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            format!("{:?}", response.guard_address),
            "0x06a85356dcb5b307096726fb86a78c59d38e08ee"
        );
    }
}
