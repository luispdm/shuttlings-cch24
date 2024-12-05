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
    from_key.from
        .octets()
        .iter()
        .zip(from_key.key.octets().iter())
        .map(|(f, k)| f.wrapping_add(*k))
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(".")
}

pub async fn key_v4(from_to: Query<SubV4>) -> String {
    from_to.to
        .octets()
        .iter()
        .zip(from_to.from.octets().iter())
        .map(|(t, f)| t.wrapping_sub(*f))
        .map(|b| b.to_string())
        .collect::<Vec<String>>()
        .join(".")
}
