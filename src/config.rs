use std::{
    fmt::Debug,
    net::{Ipv4Addr, SocketAddr},
    sync::OnceLock,
};

use figment::{providers::Env, Figment};
use serde::Deserialize;
use url::Url;

pub fn config() -> &'static Config {
    static INST: OnceLock<Config> = OnceLock::new();

    INST.get_or_init(|| Config::new())
}

fn default_port() -> u16 {
    3000
}

fn default_bind() -> Ipv4Addr {
    Ipv4Addr::new(127, 0, 0, 1)
}

fn default_db_max_conn() -> u32 {
    10
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_bind")]
    pub bind_addr: Ipv4Addr,

    pub db: Db,
    pub redis: Redis,
}

impl Config {
    /// we allow this to panic
    pub fn new() -> Self {
        Figment::new()
            .merge(Env::prefixed("AOC_").split("__"))
            .extract()
            .expect("Could not load config from ENV")
    }

    /// Get the socket address as specified by the settings.
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(std::net::IpAddr::V4(self.bind_addr), self.port)
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Db {
    #[serde(default = "default_db_max_conn")]
    pub max_connections: u32,

    url: Url,
}

impl Db {
    pub fn url(&self) -> &Url {
        &self.url
    }
}

impl Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db").field("url", &"*******").finish()
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Redis {
    pub url: Url,
}

impl Debug for Redis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redis").field("url", &"*******").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let expected = Config {
            port: 3001,
            bind_addr: Ipv4Addr::new(10, 10, 10, 15),
            db: Db {
                max_connections: 4,
                url: "postgres://aoc:fake@localhost/aoc".try_into().unwrap(),
            },
            redis: Redis {
                url: "redis://otherredis:1234".try_into().unwrap(),
            },
        };

        let config = temp_env::with_vars(
            [
                ("AOC_PORT", Some("3001")),
                ("AOC_BIND_ADDR", Some("10.10.10.15")),
                ("AOC_DB__MAX_CONNECTIONS", Some("4")),
                ("AOC_DB__URL", Some("postgres://aoc:fake@localhost/aoc")),
                ("AOC_REDIS__URL", Some("redis://otherredis:1234")),
            ],
            || Config::new(),
        );

        assert_eq!(config, expected);
    }

    #[test]
    fn debug_does_not_leak_creds() {
        let config = Config {
            port: 3001,
            bind_addr: Ipv4Addr::new(10, 10, 10, 15),
            db: Db {
                max_connections: 10,
                url: "postgres://aoc:fake@localhost/aoc".try_into().unwrap(),
            },
            redis: Redis {
                url: "redis://otherredis:1234".try_into().unwrap(),
            },
        };

        let out = format!("{:?}", &config);

        assert!(!out.contains("fake"));
    }
}
