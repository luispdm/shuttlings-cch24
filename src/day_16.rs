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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::Response};
    use http_body_util::BodyExt;
    use serde_json::json;

    async fn get_response_parts(
        response: Response,
    ) -> (StatusCode, Option<String>, Option<String>) {
        let status = response.status();
        let headers = response.headers();
        let cookie = headers
            .get(header::SET_COOKIE)
            .map(|v| v.to_str().unwrap().to_string());

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = if !body.is_empty() {
            Some(String::from_utf8(body.to_vec()).unwrap())
        } else {
            None
        };

        (status, cookie, body_str)
    }

    #[tokio::test]
    async fn test_wrap_valid_json() {
        let test_json = json!({"test": "value"});
        let response = wrap(Json(test_json)).await.into_response();
        let (status, cookie, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::OK);
        assert!(cookie.is_some());
        assert!(cookie.unwrap().starts_with(&format!("{}=", COOKIE_NAME)));
    }

    #[tokio::test]
    async fn test_wrap_complex_json() {
        let complex_json = json!({
            "string": "value",
            "number": 42,
            "boolean": true,
            "array": [1, 2, 3],
            "nested": {
                "field": "value"
            }
        });

        let response = wrap(Json(complex_json)).await.into_response();
        let (status, cookie, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::OK);
        assert!(cookie.is_some());
        assert!(cookie.unwrap().starts_with(&format!("{}=", COOKIE_NAME)));
    }

    #[tokio::test]
    async fn test_unwrap_missing_cookie() {
        let jar = CookieJar::new();
        let response = unwrap(jar).await.into_response();
        let (status, _, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_wrap_then_unwrap() {
        // wrap
        let test_json = json!({"test": "value"});
        let wrap_response = wrap(Json(test_json.clone())).await.into_response();
        let (status, cookie, _) = get_response_parts(wrap_response).await;
        assert_eq!(status, StatusCode::OK);

        // extract JWT and cookiejar
        let cookie_str = cookie.unwrap();
        let jwt = cookie_str.split('=').nth(1).unwrap();
        let jar = CookieJar::new();
        let jar = jar.add(axum_extra::extract::cookie::Cookie::new(
            COOKIE_NAME,
            jwt.to_string(),
        ));

        // unwrap
        let unwrap_response = unwrap(jar).await.into_response();
        let (status, _, body) = get_response_parts(unwrap_response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            serde_json::from_str::<Value>(&body.unwrap()).unwrap(),
            test_json
        );
    }

    #[tokio::test]
    async fn test_unwrap_invalid_jwt() {
        let jar = CookieJar::new();
        let jar = jar.add(axum_extra::extract::cookie::Cookie::new(
            COOKIE_NAME,
            "invalid.jwt.token",
        ));

        let response = unwrap(jar).await.into_response();
        let (status, _, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_decode_invalid_jwt() {
        let response = decode("invalid.jwt.token".to_string())
            .await
            .into_response();
        let (status, _, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_decode_unsupported_algorithm() {
        let test_json = json!({"test": "value"});
        let token = jsonwebtoken::encode(
            &Header::default(),
            &test_json,
            &EncodingKey::from_secret(SUPER_SECRET.as_ref()),
        )
        .unwrap();

        let response = decode(token).await.into_response();
        let (status, _, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_decode_unauthorized_rs256() {
        // random RSA key to trigger invalid signature
        let different_rsa_key = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQC7VJTUt9Us8cKj\nMzEfYyjiWA4R4/M2bS1GB4t7NXp98C3SC6dVMvDuictGeurT8jNbvJZHtCSuYEvu\nNMoSfm76oqFvAp8Gy0iz5sxjZmSnXyCdPEovGhLa0VzMaQ8s+CLOyS56YyCFGeJZ\nqgtzJ6GR3eqoYSW9b9UMvkBpZODSctWSNGj3P7jRFDO5VoTwCQAWbFnOjDfH5Ulg\np2PKSQnSJP3AJLQNFNe7br1XbrhV//eO+t51mIpGSDCUv3E0DDFcWDTH9cXDTTlR\nZVEiR2BwpZOOkE/Z0/BVnhZYL71oZV34bKfWjQIt6V/isSMahdsAASACp4ZTGtwi\nVuNd9tybAgMBAAECggEBAKTmjaS6tkK8BlPXClTQ2vpz/N6uxDeS35mXpqasqskV\nlaAidgg/sWqpjXDbXr93otIMLlWsM+X0CqMDgSXKejLS2jx4GDjI1ZTXg++0AMJ8\nsJ74pWzVDOfmCEQ/7wXs3+cbnXhKriO8Z036q92Qc1+N87SI38nkGa0ABH9CN83H\nmQqt4fB7UdHzuIRe/me2PGhIq5ZBzj6h3BpoPGzEP+x3l9YmK8t/1cN0pqI+dQwY\ndgfGjackLu/2qH80MCF7IyQaseZUOJyKrCLtSD/Iixv/hzDEUPfOCjFDgTpzf3cw\nta8+oE4wHCo1iI1/4TlPkwmXx4qSXtmw4aQPz7IDQvECgYEA8KNThCO2gsC2I9PQ\nDM/8Cw0O983WCDY+oi+7JPiNAJwv5DYBqEZB1QYdj06YD16XlC/HAZMsMku1na2T\nN0driwenQQWzoev3g2S7gRDoS/FCJSI3jJ+kjgtaA7Qmzlgk1TxODN+G1H91HW7t\n0l7VnL27IWyYo2qRRK3jzxqUiPUCgYEAx0oQs2reBQGMVZnApD1jeq7n4MvNLcPv\nt8b/eU9iUv6Y4Mj0Suo/AU8lYZXm8ubbqAlwz2VSVunD2tOplHyMUrtCtObAfVDU\nAhCndKaA9gApgfb3xw1IKbuQ1u4IF1FJl3VtumfQn//LiH1B3rXhcdyo3/vIttEk\n48RakUKClU8CgYEAzV7W3COOlDDcQd935DdtKBFRAPRPAlspQUnzMi5eSHMD/ISL\nDY5IiQHbIH83D4bvXq0X7qQoSBSNP7Dvv3HYuqMhf0DaegrlBuJllFVVq9qPVRnK\nxt1Il2HgxOBvbhOT+9in1BzA+YJ99UzC85O0Qz06A+CmtHEy4aZ2kj5hHjECgYEA\nmNS4+A8Fkss8Js1RieK2LniBxMgmYml3pfVLKGnzmng7H2+cwPLhPIzIuwytXywh\n2bzbsYEfYx3EoEVgMEpPhoarQnYPukrJO4gwE2o5Te6T5mJSZGlQJQj9q4ZB2Dfz\net6INsK0oG8XVGXSpQvQh3RUYekCZQkBBFcpqWpbIEsCgYAnM3DQf3FJoSnXaMhr\nVBIovic5l0xFkEHskAjFTevO86Fsz1C2aSeRKSqGFoOQ0tmJzBEs1R6KqnHInicD\nTQrKhArgLXX4v3CddjfTRJkFWDbE/CkvKZNOrcf1nhaGCPspRJj2KUkj1Fhl9Cnc\ndn/RsYEONbwQSjIfMPkvxF+8HQ==\n-----END PRIVATE KEY-----\n";

        let test_json = json!({"test": "value"});
        let header = Header::new(Algorithm::RS256);

        let token = jsonwebtoken::encode(
            &header,
            &test_json,
            &EncodingKey::from_rsa_pem(different_rsa_key.as_bytes()).unwrap(),
        )
        .unwrap();

        let response = decode(token).await.into_response();
        let (status, _, _) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_decode_valid_rs256() {
        let test_json = json!({"reindeerSnack":"carrots","santaHatColor":"red","snowGlobeCollection":5,"stockingStuffers":["yo-yo","candy","keychain"],"treeHeight":7});

        let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJyZWluZGVlclNuYWNrIjoiY2Fycm90cyIsInNhbnRhSGF0Q29sb3IiOiJyZWQiLCJzbm93R2xvYmVDb2xsZWN0aW9uIjo1LCJzdG9ja2luZ1N0dWZmZXJzIjpbInlvLXlvIiwiY2FuZHkiLCJrZXljaGFpbiJdLCJ0cmVlSGVpZ2h0Ijo3fQ.EoWSlwZIMHdtd96U_FkfQ9SkbzskSvgEaRpsUeZQFJixDW57vZud_k-MK1R1LEGoJRPGttJvG_5ewdK9O46OuaGW4DHIOWIFLxSYFTJBdFMVmAWC6snqartAFr2U-LWxTwJ09WNpPBcL67YCx4HQsoGZ2mxRVNIKxR7IEfkZDhmpDkiAUbtKyn0H1EVERP1gdbzHUGpLd7wiuzkJnjenBgLPifUevxGPgj535cp8I6EeE4gLdMEm3lbUW4wX_GG5t6_fDAF4URfiAOkSbiIW6lKcSGD9MBVEGps88lA2REBEjT4c7XHw4Tbxci2-knuJm90zIA9KX92t96tF3VFKEA";

        let response = decode(token.to_string()).await.into_response();
        let (status, _, body) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            serde_json::from_str::<Value>(&body.unwrap()).unwrap(),
            test_json
        );
    }

    #[tokio::test]
    async fn test_decode_valid_rs512() {
        let test_json = json!({"candleScents":["pine","cinnamon","vanilla"],"festiveSocks":12,"giftTags":["personalized","blank","sparkly"],"gingerbreadHouseKits":3,"hotCocoaStock":25});

        let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJjYW5kbGVTY2VudHMiOlsicGluZSIsImNpbm5hbW9uIiwidmFuaWxsYSJdLCJmZXN0aXZlU29ja3MiOjEyLCJnaWZ0VGFncyI6WyJwZXJzb25hbGl6ZWQiLCJibGFuayIsInNwYXJrbHkiXSwiZ2luZ2VyYnJlYWRIb3VzZUtpdHMiOjMsImhvdENvY29hU3RvY2siOjI1fQ.GgYB9NXomy-s_lzmoRC-BFHUvrSMjDMcZ4jFCre6NaPJA2fKr--cadxerpody-H5wV19N2zguNb5gr6dt7-suegC8D2ANe9mExohY9tuqgGKRJdLqtmb8U91T_iRg2kyAyhrv3HlSUHQP3sxvAO7jcwLtbePQehtzb6Hv9tZqNCojxMJmAhrJxz41fnD9wvTsEZVpQVwo21C-GIpZKRUGJnaL6OU9IAY6D4PMUr4X9OjEC1zSdQWpYUW_8CHrGNYPVg-6ZpdEvkejxZGTwPg8pMPPSxRa6g0v7Scx-50pgjcP15VK2OUaF9xce7MReJOgI2dxtF35DpYT-UNsIWDKg";

        let response = decode(token.to_string()).await.into_response();
        let (status, _, body) = get_response_parts(response).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            serde_json::from_str::<Value>(&body.unwrap()).unwrap(),
            test_json
        );
    }
}
