use axum::response::{IntoResponse, Response};
use hyper::http::StatusCode;

pub mod create_voucher;

const HUB_URL: &str = "https://testnet.hub.snapshot.org/graphql";

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
