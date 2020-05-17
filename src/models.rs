use anyhow::Result;
use serde::Deserialize;
use std::env;

/// Struct that contains the required information to
/// access the Reddit API.
#[derive(Clone, Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub user_agent: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Config {
    /// Pulls data from environment variables to populate the struct.
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            username: env::var("CFL_USERNAME")?,
            password: env::var("CFL_PASSWORD")?,
            user_agent: env::var("CFL_USER_AGENT")?,
            client_id: env::var("CFL_CLIENT_ID")?,
            client_secret: env::var("CFL_CLIENT_SECRET")?,
        })
    }
}

/// Typed response from Reddit's login endpoint.
#[derive(Debug, Deserialize, PartialEq)]
pub struct AccessTokenResponse {
    #[serde(alias = "access_token")]
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
}
