use std::collections::HashSet;

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::Value;

const COOKIE_NAME: &str = "gift";
const SUPER_SECRET: &str = "perkele-santa";
const RSA_PEM: &str = include_str!("./day_16/rsa.pem");

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

    let mut res = decode_with_algorithm(
        &jwt,
        &DecodingKey::from_secret(SUPER_SECRET.as_ref()),
        Algorithm::HS256,
    );
    if res.0 == StatusCode::UNAUTHORIZED {
        res.0 = StatusCode::BAD_REQUEST;
    }
    res
}

pub async fn decode(jwt: String) -> impl IntoResponse {
    let decoding_key = match DecodingKey::from_rsa_pem(RSA_PEM.as_ref()) {
        Ok(key) => key,
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "".to_string()),
    };

    let algorithm = match jsonwebtoken::decode_header(&jwt) {
        Ok(header) => match header.alg {
            Algorithm::RS256 => Algorithm::RS256,
            Algorithm::RS384 => Algorithm::RS384,
            Algorithm::RS512 => Algorithm::RS512,
            _ => return (StatusCode::BAD_REQUEST, "".to_string()),
        },
        _ => return (StatusCode::BAD_REQUEST, "".to_string()),
    };

    decode_with_algorithm(&jwt, &decoding_key, algorithm)
}

fn decode_with_algorithm(
    jwt: &str,
    decoding_key: &DecodingKey,
    algorithm: Algorithm,
) -> (StatusCode, String) {
    let mut validation = Validation::new(algorithm);
    validation.required_spec_claims = HashSet::new();

    match jsonwebtoken::decode::<Value>(jwt, decoding_key, &validation) {
        Ok(token) => (StatusCode::OK, token.claims.to_string()),
        Err(e) => match e.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "".to_string())
            }
            _ => (StatusCode::BAD_REQUEST, "".to_string()),
        },
    }
}
