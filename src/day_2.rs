use std::net::Ipv4Addr;

use axum::extract::Query;

#[derive(Debug, serde::Deserialize)]
pub struct AddV4 {
    from: Ipv4Addr,
    key: Ipv4Addr,
}

#[derive(Debug, serde::Deserialize)]
pub struct SubV4 {
    from: Ipv4Addr,
    to: Ipv4Addr,
}

pub async fn dest_v4(from_key: Query<AddV4>) -> String {
    from_key
        .from
        .octets()
        .iter()
        .zip(from_key.key.octets())
        .map(|(f, k)| f.wrapping_add(k).to_string())
        .collect::<Vec<String>>()
        .join(".")
}

pub async fn key_v4(from_to: Query<SubV4>) -> String {
    from_to
        .to
        .octets()
        .iter()
        .zip(from_to.from.octets())
        .map(|(t, f)| t.wrapping_sub(f).to_string())
        .collect::<Vec<String>>()
        .join(".")
}
