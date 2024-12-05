use std::net::{Ipv4Addr, Ipv6Addr};

use axum::extract::Query;

#[derive(Debug, serde::Deserialize)]
pub struct DestV4 {
    from: Ipv4Addr,
    key: Ipv4Addr,
}

#[derive(Debug, serde::Deserialize)]
pub struct KeyV4 {
    from: Ipv4Addr,
    to: Ipv4Addr,
}

#[derive(Debug, serde::Deserialize)]
pub struct DestV6 {
    from: Ipv6Addr,
    key: Ipv6Addr,
}

#[derive(Debug, serde::Deserialize)]
pub struct KeyV6 {
    from: Ipv6Addr,
    to: Ipv6Addr,
}

pub async fn dest_v4(from_key: Query<DestV4>) -> String {
    from_key
        .from
        .octets()
        .iter()
        .zip(from_key.key.octets())
        .map(|(f, k)| f.wrapping_add(k).to_string())
        .collect::<Vec<String>>()
        .join(".")
}

pub async fn key_v4(from_to: Query<KeyV4>) -> String {
    from_to
        .to
        .octets()
        .iter()
        .zip(from_to.from.octets())
        .map(|(t, f)| t.wrapping_sub(f).to_string())
        .collect::<Vec<String>>()
        .join(".")
}

pub async fn dest_v6(from_key: Query<DestV6>) -> String {
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

pub async fn key_v6(from_to: Query<KeyV6>) -> String {
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
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dest_v4() {
        let from_key = Query(DestV4 {
            from: Ipv4Addr::new(192, 168, 1, 1),
            key: Ipv4Addr::new(10, 0, 0, 1),
        });
        let result = dest_v4(from_key).await;
        assert_eq!(result, "202.168.1.2");
    }

    #[tokio::test]
    async fn test_key_v4() {
        let from_to = Query(KeyV4 {
            from: Ipv4Addr::new(192, 168, 1, 1),
            to: Ipv4Addr::new(202, 168, 1, 2),
        });
        let result = key_v4(from_to).await;
        assert_eq!(result, "10.0.0.1");
    }

    #[tokio::test]
    async fn test_dest_v6() {
        let from_key = Query(DestV6 {
            from: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            key: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2),
        });
        let result = dest_v6(from_key).await;
        assert_eq!(result, "::3");
    }

    #[tokio::test]
    async fn test_key_v6() {
        let from_to = Query(KeyV6 {
            from: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            to: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 3),
        });
        let result = key_v6(from_to).await;
        assert_eq!(result, "2001:db8::2");
    }

    #[tokio::test]
    async fn test_dest_v4_zero() {
        let from_key = Query(DestV4 {
            from: Ipv4Addr::new(0, 0, 0, 0),
            key: Ipv4Addr::new(0, 0, 0, 0),
        });
        let result = dest_v4(from_key).await;
        assert_eq!(result, "0.0.0.0");
    }

    #[tokio::test]
    async fn test_key_v4_zero() {
        let from_to = Query(KeyV4 {
            from: Ipv4Addr::new(0, 0, 0, 0),
            to: Ipv4Addr::new(0, 0, 0, 0),
        });
        let result = key_v4(from_to).await;
        assert_eq!(result, "0.0.0.0");
    }

    #[tokio::test]
    async fn test_dest_v6_zero() {
        let from_key = Query(DestV6 {
            from: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            key: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
        });
        let result = dest_v6(from_key).await;
        assert_eq!(result, "::");
    }

    #[tokio::test]
    async fn test_key_v6_zero() {
        let from_to = Query(KeyV6 {
            from: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
            to: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
        });
        let result = key_v6(from_to).await;
        assert_eq!(result, "::");
    }

    #[tokio::test]
    async fn test_dest_v4_overflow() {
        let from_key = Query(DestV4 {
            from: Ipv4Addr::new(255, 255, 255, 255),
            key: Ipv4Addr::new(255, 255, 255, 255),
        });
        let result = dest_v4(from_key).await;
        assert_eq!(result, "254.254.254.254");
    }

    #[tokio::test]
    async fn test_key_v4_overflow() {
        let from_to = Query(KeyV4 {
            from: Ipv4Addr::new(255, 255, 255, 255),
            to: Ipv4Addr::new(254, 254, 254, 254),
        });
        let result = key_v4(from_to).await;
        assert_eq!(result, "255.255.255.255");
    }

    #[tokio::test]
    async fn test_dest_v6_max() {
        let from_key = Query(DestV6 {
            from: Ipv6Addr::new(
                0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
            ),
            key: Ipv6Addr::new(
                0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
            ),
        });
        let result = dest_v6(from_key).await;
        assert_eq!(result, "::");
    }

    #[tokio::test]
    async fn test_key_v6_max() {
        let from_to = Query(KeyV6 {
            from: Ipv6Addr::new(
                0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
            ),
            to: Ipv6Addr::new(
                0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
            ),
        });
        let result = key_v6(from_to).await;
        assert_eq!(result, "::");
    }
}
