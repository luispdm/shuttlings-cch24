use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cargo_manifest::Manifest;
use serde::Deserialize;
use serde_with::serde_as;

#[derive(Default, Debug, Deserialize)]
struct Metadata {
    #[serde(default)]
    orders: Vec<Order>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct Order {
    item: String,
    #[serde_as(deserialize_as = "serde_with::DefaultOnError")]
    #[serde(default)]
    quantity: Option<u32>,
}

const MAGIC_KEYWORD: &str = "Christmas 2024";

#[axum::debug_handler]
pub async fn manifest(headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    // parsing body depending on content-type
    let maybe_package = match headers.get("Content-Type") {
        Some(ct) if ct == "application/toml" => {
            Manifest::<Metadata>::from_slice_with_metadata(&body).ok()
        }
        Some(ct) if ct == "application/yaml" => {
            serde_yml::from_slice::<Manifest<Metadata>>(&body).ok()
        }
        Some(ct) if ct == "application/json" => {
            serde_json::from_slice::<Manifest<Metadata>>(&body).ok()
        }
        _ => {
            return Response::builder()
                .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
                .body("".to_string())
                .unwrap();
        }
    }
    .and_then(|m| m.package);

    // invalid manifest - 400
    if maybe_package.is_none() {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Invalid manifest".to_string())
            .unwrap();
    }

    let package = maybe_package.unwrap();

    // no magic keyword provided - 400
    if !package
        .keywords
        .and_then(|maybe_keys| maybe_keys.as_local())
        .map(|keys| keys.contains(&MAGIC_KEYWORD.to_string()))
        .unwrap_or(false)
    {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Magic keyword not provided".to_string())
            .unwrap();
    }

    // collect potential orders
    let response = package
        .metadata
        .map(|metadata| {
            metadata
                .orders
                .iter()
                .filter_map(|o| o.quantity.map(|q| format!("{}: {}", o.item, q)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|r| !r.is_empty());

    // 204 with no orders, 200 with orders
    match response {
        Some(r) => Response::builder()
            .status(StatusCode::OK)
            .body(r.trim().to_string())
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body("".to_string())
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use axum::body::Body;
    use http_body_util::BodyExt;

    async fn body_to_string(body: Body) -> String {
        let collected = body.collect().await.unwrap();
        let body_bytes = collected.to_bytes();
        String::from_utf8(body_bytes.to_vec()).unwrap()
    }

    fn create_headers(content_type: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
        headers
    }

    #[tokio::test]
    async fn test_valid_toml_manifest() {
        let toml_content = r#"
        [package]
        name = "not-a-gift-order"
        authors = ["Not Santa"]
        keywords = ["Christmas 2024"]

        [[package.metadata.orders]]
        item = "Toy Car"
        quantity = 5

        [[package.metadata.orders]]
        item = "Doll"
        quantity = 3
        "#;

        let headers = create_headers("application/toml");
        let response = manifest(headers, Bytes::from(toml_content.as_bytes().to_vec())).await;
        
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        
        let body_content = body_to_string(body).await;
        assert_eq!(body_content, "Toy Car: 5\nDoll: 3");
    }

    #[tokio::test]
    async fn test_valid_yaml_manifest() {
        let yaml_content = r#"
        package:
          name: not-a-gift-order
          authors: ["Not Santa"]
          keywords: ["Christmas 2024"]
          metadata:
            orders:
              - item: Bicycle
                quantity: 2
              - item: Skateboard
                quantity: 1
        "#;

        let headers = create_headers("application/yaml");
        let response = manifest(headers, Bytes::from(yaml_content.as_bytes().to_vec())).await;
        
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        
        let body_content = body_to_string(body).await;
        assert_eq!(body_content, "Bicycle: 2\nSkateboard: 1");
    }

    #[tokio::test]
    async fn test_valid_json_manifest() {
        let json_content = r#"{
            "package": {
                "name": "not-a-gift-order",
                "authors": ["Not Santa"],
                "keywords": ["Christmas 2024"],
                "metadata": {
                    "orders": [
                        {"item": "Puzzle", "quantity": 4},
                        {"item": "Book", "quantity": 3}
                    ]
                }
            }
        }"#;

        let headers = create_headers("application/json");
        let response = manifest(headers, Bytes::from(json_content.as_bytes().to_vec())).await;
        
        let (parts, body) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        
        let body_content = body_to_string(body).await;
        assert_eq!(body_content, "Puzzle: 4\nBook: 3");
    }

    #[tokio::test]
    async fn test_missing_magic_keyword() {
        let toml_content = r#"
        [package]
        keywords = ["Holiday"]

        [package.metadata]
        [[package.metadata.orders]]
        item = "Toy Car"
        quantity = 5
        "#;

        let headers = create_headers("application/toml");
        let response = manifest(headers, Bytes::from(toml_content.as_bytes().to_vec())).await;
        
        let (parts, _) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_no_keywords() {
        let toml_content = r#"
        [package]

        [package.metadata]
        [[package.metadata.orders]]
        item = "Toy Car"
        quantity = 5
        "#;

        let headers = create_headers("application/toml");
        let response = manifest(headers, Bytes::from(toml_content.as_bytes().to_vec())).await;
        
        let (parts, _) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_no_orders() {
        let toml_content = r#"
        [package]
        name = "not-a-gift-order"
        authors = ["Not Santa"]
        keywords = ["Christmas 2024"]

        [package.metadata]
        orders = []
        "#;

        let headers = create_headers("application/toml");
        let response = manifest(headers, Bytes::from(toml_content.as_bytes().to_vec())).await;
        
        let (parts, _) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_orders_without_quantity() {
        let toml_content = r#"
        [package]
        name = "not-a-gift-order"
        authors = ["Not Santa"]
        keywords = ["Christmas 2024"]

        [package.metadata]
        [[package.metadata.orders]]
        item = "Toy Car"

        [[package.metadata.orders]]
        item = "Doll"
        "#;

        let headers = create_headers("application/toml");
        let response = manifest(headers, Bytes::from(toml_content.as_bytes().to_vec())).await;
        
        let (parts, _) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_unsupported_content_type() {
        let content = "Some content";
        let headers = create_headers("application/xml");
        let response = manifest(headers, Bytes::from(content.as_bytes().to_vec())).await;
        
        let (parts, _) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_invalid_manifest_format() {
        let invalid_content = "Invalid manifest content";
        let headers = create_headers("application/json");
        let response = manifest(headers, Bytes::from(invalid_content.as_bytes().to_vec())).await;
        
        let (parts, _) = response.into_response().into_parts();
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    }
}
