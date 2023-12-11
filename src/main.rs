use axum::routing::post;
use axum::Router;
use boost_guard::create_voucher::create_voucher_handler;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    Router::new().route("/create_voucher", post(create_voucher_handler))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http;
    use boost_guard::create_voucher::{CreateVoucherParams, CreateVoucherResponse};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_voucher_handler() {
        let app = super::app();
        let query = CreateVoucherParams {
            proposal_id: "0x5228df2f6851e31cf2d4cc4d3f1d46fc79fb33760caafbf6aae6a3321694aa01"
                .to_string(),
            voter_address: "voter_address".to_string(),
            boost_id: "boost".to_string(),
        };

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .uri("/create_voucher")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&[&query, &query]).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let _response: CreateVoucherResponse =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
    }

    #[tokio::test]
    async fn test_invalid_proposal_type() {
        let app = super::app();
        let query = CreateVoucherParams {
            proposal_id: "0x2f488ec3a0b9b5d731812395f2aa99718df7d380b6c6c0539fec16ae53b3e1fc"
                .to_string(),
            voter_address: "voter_address".to_string(),
            boost_id: "boost".to_string(),
        };

        let response = app
            .oneshot(
                http::Request::builder()
                    .method(http::Method::POST)
                    .uri("/create_voucher")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(serde_json::to_vec(&[&query, &query]).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status() == http::StatusCode::INTERNAL_SERVER_ERROR);
    }
}
