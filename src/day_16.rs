use std::collections::HashSet;

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde_json::Value;

const COOKIE_NAME: &str = "gift";
const SUPER_SECRET: &str = "perkele-santa";

pub async fn wrap(Json(body): Json<Value>) -> impl IntoResponse {
    match jsonwebtoken::encode(
        &Header::default(),
        &body,
        &EncodingKey::from_secret(SUPER_SECRET.as_ref()),
    ) {
        Ok(token) => (
            StatusCode::OK,
            [(header::SET_COOKIE, format!("{}={}", COOKIE_NAME, token))],
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
        ),
    }
}

pub async fn unwrap(jar: CookieJar) -> impl IntoResponse {
    let jwt = match jar.get(COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        _ => return (StatusCode::BAD_REQUEST, "".to_string()),
    };

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.required_spec_claims = HashSet::new();

    match jsonwebtoken::decode::<Value>(
        &jwt,
        &DecodingKey::from_secret(SUPER_SECRET.as_ref()),
        &validation,
    ) {
        Ok(token) => (StatusCode::OK, token.claims.to_string()),
        _ => (StatusCode::BAD_REQUEST, "".to_string()),
    }
}

pub async fn decode() -> impl IntoResponse {
    "todo"
}
