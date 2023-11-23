use std::{
    fmt::Debug,
    net::{Ipv4Addr, SocketAddr},
    ops::Deref,
    sync::OnceLock,
};

use argon2::Argon2;
use figment::{providers::Env, Figment};
use password_hash::PasswordHashString;
use serde::{de::Visitor, Deserialize};
use url::Url;

pub fn config() -> &'static Config {
    static INST: OnceLock<Config> = OnceLock::new();

    // there has to be a better way to do this that doesn't involve setting the
    // env vars globally or in every test case
    if cfg!(test) {
        INST.get_or_init(|| Config {
            port: 3001,
            bind_addr: Ipv4Addr::new(10, 10, 10, 15),
            db: Db {
                max_connections: 4,
                url: "postgres://aoc:sandcastle@localhost/aoc"
                    .try_into()
                    .unwrap(),
            },
            redis: Redis {
                url: "redis://otherredis:1234".try_into().unwrap(),
            },
            secret: Secret {
                api_token: HashWrapper(
                    PasswordHashString::new(
                        "$argon2id$v=19$m=19,t=2,p=1$cnBVTU1hTnA3SWppYk56bQ$h9WU9gybGvxV6TUA46S96w",
                    )
                    .unwrap(),
                ),
            },
        })
    } else {
        INST.get_or_init(Config::new)
    }
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
    pub secret: Secret,
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

impl Default for Config {
    fn default() -> Self {
        Self::new()
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

// we need to get aroudn the fact that we don't own this
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashWrapper(PasswordHashString);

impl Deref for HashWrapper {
    type Target = PasswordHashString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Secret {
    api_token: HashWrapper,
}

impl Secret {
    // it's maybe clunky to have this here but I didn't want to pass around the
    // token, even if it's a hash
    pub fn validate(&self, input: &str) -> bool {
        self.api_token
            .password_hash()
            .verify_password(&[&Argon2::default()], input)
            .is_ok()
    }
}

impl Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secret")
            .field("api_token", &"*******")
            .finish()
    }
}

impl<'de> Deserialize<'de> for HashWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(CustomVisitor)
    }
}

struct CustomVisitor;

impl<'de> Visitor<'de> for CustomVisitor {
    type Value = HashWrapper;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a valid argon2 hash")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let h = PasswordHashString::new(v)
            .map_err(|e| serde::de::Error::custom(format!("Invalid hash: {:?}", e)))?;

        Ok(HashWrapper(h))
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
            secret: Secret {
                api_token: HashWrapper(
                    PasswordHashString::new(
                        "$argon2id$v=19$m=19,t=2,p=1$cnBVTU1hTnA3SWppYk56bQ$h9WU9gybGvxV6TUA46S96w",
                    )
                    .unwrap(),
                ),
            },
        };

        let config = temp_env::with_vars(
            [
                ("AOC_PORT", Some("3001")),
                ("AOC_BIND_ADDR", Some("10.10.10.15")),
                ("AOC_DB__MAX_CONNECTIONS", Some("4")),
                ("AOC_DB__URL", Some("postgres://aoc:fake@localhost/aoc")),
                ("AOC_REDIS__URL", Some("redis://otherredis:1234")),
                (
                    "AOC_SECRET__API_TOKEN",
                    Some(
                        "$argon2id$v=19$m=19,t=2,p=1$cnBVTU1hTnA3SWppYk56bQ$h9WU9gybGvxV6TUA46S96w",
                    ),
                ),
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
            secret: Secret {
                api_token: HashWrapper(
                    PasswordHashString::new(
                        "$argon2id$v=19$m=19,t=2,p=1$cnBVTU1hTnA3SWppYk56bQ$h9WU9gybGvxV6TUA46S96w",
                    )
                    .unwrap(),
                ),
            },
        };

        let out = format!("{:?}", &config);

        assert!(!out.contains("fake"));
    }

    #[test]
    fn secret_validation() {
        let secret = Secret {
            api_token: HashWrapper(
                PasswordHashString::new(
                    "$argon2id$v=19$m=19,t=2,p=1$cnBVTU1hTnA3SWppYk56bQ$h9WU9gybGvxV6TUA46S96w",
                )
                .unwrap(),
            ),
        };

        assert!(secret.validate("sandcastle"));
        assert!(!secret.validate("not-sandcastle"));
    }
}
