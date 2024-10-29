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

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    let _ = tracing::subscriber::set_global_default(subscriber);
    tracing::info!("Starting server...");

    let port: u16 = env::var("PORT")
        .map(|val| val.parse::<u16>().unwrap())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    dotenv().ok();

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

    const WINNER: &str = "0x3901D0fDe202aF1427216b79f5243f8A022d68cf";
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

    #[tokio::test]
    async fn test_get_rewards_ranked_choice() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0x930d5fb011f84d16df26c362d820323f0dab111c3b0b91d75151fe12c5ff07fb"
                .to_string(),
            voter_address: "0x5EF29cf961cf3Fc02551B9BdaDAa4418c446c5dd".to_string(),
            boosts: vec![
                ("42".to_string(), "11155111".to_string()),
                ("43".to_string(), "11155111".to_string()),
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
            let result = response.unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].reward, "15000000000000000000");
            assert_eq!(result[0].chain_id, "11155111");
            assert_eq!(result[0].boost_id, "43");
        }
    }

    #[tokio::test]
    async fn test_get_rewards_shutter() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0xdde52de1d892ccc671dcca55504803f87a2297089fd728ef2076af4c1b96ac1c"
                .to_string(),
            voter_address: "0x5EF29cf961cf3Fc02551B9BdaDAa4418c446c5dd".to_string(),
            boosts: vec![
                ("44".to_string(), "11155111".to_string()),
                ("45".to_string(), "11155111".to_string()),
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
            let result = response.unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].reward, "15000000000000000000");
            assert_eq!(result[0].chain_id, "11155111");
            assert_eq!(result[0].boost_id, "45");
        }
    }

    #[tokio::test]
    async fn test_get_rewards_shutter_and_ranked_choice() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0xfcdb01284958142a481fb4d579aa056ed93c29a9f58fbefbfb0504b3c1c06e96"
                .to_string(),
            voter_address: "0xc83A9e69012312513328992d454290be85e95101".to_string(),
            boosts: vec![
                ("46".to_string(), "11155111".to_string()),
                ("47".to_string(), "11155111".to_string()),
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
            let result = response.unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].reward, "15000000000000000000");
            assert_eq!(result[0].chain_id, "11155111");
            assert_eq!(result[0].boost_id, "47");
        }
    }

    #[tokio::test]
    async fn test_get_rewards_shutter_and_ranked_proportional() {
        let app = super::app();
        let query = QueryParams {
            proposal_id: "0xe175412d46744bdb68e61c89492a5d3ebb55a487cf8fc4d35a0d671302babed3"
                .to_string(),
            voter_address: "0xc83A9e69012312513328992d454290be85e95101".to_string(),
            boosts: vec![("49".to_string(), "11155111".to_string())],
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
            assert_eq!(result[0].reward, "15000000000000000000");
            assert_eq!(result[0].chain_id, "11155111");
            assert_eq!(result[0].boost_id, "49");
        }
    }
}
