use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Package {
    checksum: Option<String>,
}

#[derive(Deserialize)]
struct Lockfile {
    package: Vec<Package>,
}

fn escape_string(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
        .replace("/", "&#x2F;")
}

pub async fn star() -> impl IntoResponse {
    (StatusCode::OK, "<div id=\"star\" class=\"lit\"></div>")
}

pub async fn present(Path(color): Path<String>) -> impl IntoResponse {
    let color = escape_string(&color);

    let (class, next) = match color {
        c if c == "blue" => ("blue", "purple"),
        c if c == "purple" => ("purple", "red"),
        c if c == "red" => ("red", "blue"),
        _ => return (StatusCode::IM_A_TEAPOT, "".to_string()),
    };

    let div = format!(
        "<div class=\"present {}\" hx-get=\"/23/present/{}\" hx-swap=\"outerHTML\"> <div class=\"ribbon\"></div><div class=\"ribbon\"></div><div class=\"ribbon\"></div><div class=\"ribbon\"></div></div>",
        class, next);

    (StatusCode::OK, div)
}

pub async fn ornament(Path((state, number)): Path<(String, String)>) -> impl IntoResponse {
    let (state, number) = (escape_string(&state), escape_string(&number));

    let (class, next_state) = match state {
        s if s == "on" => (" on", "off"),
        s if s == "off" => ("", "on"),
        _ => return (StatusCode::IM_A_TEAPOT, "".to_string()),
    };

    let div = format!(
        "<div class=\"ornament{}\" id=\"ornament{}\" hx-trigger=\"load delay:2s once\" hx-get=\"/23/ornament/{}/{}\" hx-swap=\"outerHTML\"></div>",
        class, number, next_state, number
    );

    (StatusCode::OK, div)
}

pub async fn lockfile(multipart: Multipart) -> Result<String, (StatusCode, String)> {
    let lockfile: Lockfile = parse_lockfile(multipart).await?;
    let mut res = String::new();
    for p in lockfile.package {
        if let Some(checksum) = p.checksum {
            let div = div_from_checksum(checksum)?;
            res.push_str(&div);
        };
    }

    Ok(res)
}

async fn parse_lockfile(mut multipart: Multipart) -> Result<Lockfile, (StatusCode, String)> {
    let field = match multipart
        .next_field()
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "".to_string()))?
    {
        Some(f) => f,
        _ => return Err((StatusCode::BAD_REQUEST, "".to_string())),
    };

    let text_field = field
        .text()
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "".to_string()))?;

    let lockfile = toml::from_str::<Lockfile>(&text_field)
        .map_err(|_| (StatusCode::BAD_REQUEST, "".to_string()))?;
    Ok(lockfile)
}

fn div_from_checksum(checksum: String) -> Result<String, (StatusCode, String)> {
    if checksum.len() < 10 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "".to_string()));
    }

    // making sure that color is a hex string
    u64::from_str_radix(&checksum[..6], 16)
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "".to_string()))?;
    let top = u64::from_str_radix(&checksum[6..8], 16)
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "".to_string()))?;
    let left = u64::from_str_radix(&checksum[8..10], 16)
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "".to_string()))?;

    // color is printed as-is to include leading zeros
    Ok(format!(
        "<div style=\"background-color:#{};top:{}px;left:{}px;\"></div>",
        &checksum[0..6],
        top,
        left
    ))
}
