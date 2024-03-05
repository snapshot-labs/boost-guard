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

    const WINNER: &str = "0xeF8305E140ac520225DAf050e2f71d5fBcC543e7";
    const PROPOSAL_ID: &str = "0x9f71aae9f1444d97bd4291a15820bf3f5578edfa9c41b45277e97b1d997cecf1";
    const BOOST_ID: &str = "4";
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
            assert_eq!(result[0].signature, "0x8e91dfd90ed6636c492af00a01435e0d29f7b770a02199d423cf4fae006868465cabcd2cbfa70612900cba371d52ccc63bc99ea96fd7891ce9770e28c0cce71f1b");
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
            assert_eq!(
                result.winners[0],
                "0xef8305e140ac520225daf050e2f71d5fbcc543e7"
            );
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
