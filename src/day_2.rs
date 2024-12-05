use std::net::{Ipv4Addr, Ipv6Addr};

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

#[derive(Debug, serde::Deserialize)]
pub struct AddV6 {
    from: Ipv6Addr,
    key: Ipv6Addr,
}

#[derive(Debug, serde::Deserialize)]
pub struct SubV6 {
    from: Ipv6Addr,
    to: Ipv6Addr,
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

pub async fn dest_v6(from_key: Query<AddV6>) -> String {
    let segments: [u16; 8] = from_key
        .from
        .segments()
        .iter()
        .zip(from_key.key.segments())
        .map(|(f, k)| f ^ k)
        .collect::<Vec<u16>>()
        .try_into()
        .unwrap();
    Ipv6Addr::from(segments).to_string()
}

pub async fn key_v6(from_to: Query<SubV6>) -> String {
    let segments: [u16; 8] = from_to
        .to
        .segments()
        .iter()
        .zip(from_to.from.segments())
        .map(|(t, f)| t ^ f)
        .collect::<Vec<u16>>()
        .try_into()
        .unwrap();
    Ipv6Addr::from(segments).to_string()
}
