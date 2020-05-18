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
    pub github_username: String,
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
            github_username: env::var("CFL_GITHUB_USERNAME")?,
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

#[cfg(test)]
mod tests {
    use super::{AccessTokenResponse, Config};
    use std::env;

    #[test]
    fn config_from_env() {
        env::set_var("CFL_USERNAME", "a");
        env::set_var("CFL_PASSWORD", "b");
        env::set_var("CFL_USER_AGENT", "c");
        env::set_var("CFL_CLIENT_ID", "d");
        env::set_var("CFL_CLIENT_SECRET", "e");
        env::set_var("CFL_GITHUB_USERNAME", "f");

        let c = Config::from_env().unwrap();

        assert_eq!(c.username, "a");
        assert_eq!(c.password, "b");
        assert_eq!(c.user_agent, "c");
        assert_eq!(c.client_id, "d");
        assert_eq!(c.client_secret, "e");
        assert_eq!(c.github_username, "f");
    }

    #[test]
    fn access_token_from_json() {
        let s = r#"{"access_token":"a","token_type":"b","expires_in":1,"scope":"c"}"#;
        let a: AccessTokenResponse = serde_json::from_str(s).unwrap();

        assert_eq!(a.token, "a");
        assert_eq!(a.token_type, "b");
        assert_eq!(a.expires_in, 1);
        assert_eq!(a.scope, "c");
    }
}
