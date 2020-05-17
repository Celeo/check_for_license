use anyhow::{anyhow, Result};
use log::debug;
use reqwest::{header, Client, ClientBuilder};
use std::{collections::HashMap, time};

use crate::models::{AccessTokenResponse, Config};

/// Struct that encapsulates all API-interaction logic.
#[derive(Debug)]
pub struct Bot {
    config: Config,
    client: Client,
    access_token: Option<String>,
}

fn build_client(config: &Config, acccess_token: Option<String>) -> Result<Client> {
    let mut builder = ClientBuilder::new()
        .user_agent(&config.user_agent)
        .timeout(time::Duration::from_secs(60));
    if let Some(t) = acccess_token {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("bearer {}", t))?,
        );
        builder = builder.default_headers(headers);
    }
    Ok(builder.build()?)
}

impl Bot {
    /// Create a new bot from a `Config`.
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            client: build_client(&config, None)?,
            access_token: None,
        })
    }

    /// Logs the bot in.
    ///
    /// Must be called before making any authenticated calls.
    pub async fn login(&mut self) -> Result<()> {
        debug!("Performing bot login");
        let base_url = "https://www.reddit.com";
        let form = {
            let mut form = HashMap::new();
            form.insert("grant_type", "password");
            form.insert("username", &self.config.username);
            form.insert("password", &self.config.password);
            form
        };
        let resp = self
            .client
            .post(&format!("{}/api/v1/access_token", base_url))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&form)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "Got status code {} from login attempt",
                resp.status()
            ));
        }
        let data = resp.json::<AccessTokenResponse>().await?;
        debug!("ATR from API: {:?}", data);
        self.client = build_client(&self.config, Some(data.token))?;

        Ok(())
    }
}
