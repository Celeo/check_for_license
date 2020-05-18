use anyhow::{anyhow, Result};
use log::{debug, error};
use reqwest::{header, Client, ClientBuilder};
use serde_json::Value;
use std::{collections::HashMap, fs, time};
use tokio::time::delay_for;

use crate::models::{AccessTokenResponse, Config};
use crate::util::extract_gh_info;

const BASE_URL: &str = "https://www.reddit.com";
const OAUTH_URL: &str = "https://oauth.reddit.com";
const RESPONSE_TEXT: &str = r#"The linked GitHub repository does not contain a license.

Please read over this article for more information: https://help.github.com/en/github/creating-cloning-and-archiving-repositories/licensing-a-repository"#;
const EMPTY_SUBREDDIT_DELAY: u64 = 15;

/// Struct that encapsulates all API-interaction logic.
#[derive(Debug)]
pub struct Bot {
    config: Config,
    reddit_client: Client,
    github_client: Client,
    access_token: Option<String>,
    processed: Vec<String>,
}

/// Build a `reqwest::Client`.
fn build_client(config: &Config, access_token: Option<String>) -> Result<Client> {
    let mut builder = ClientBuilder::new()
        .user_agent(&config.user_agent)
        .timeout(time::Duration::from_secs(60));
    if let Some(t) = access_token {
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
            reddit_client: build_client(&config, None)?,
            github_client: ClientBuilder::new()
                .timeout(time::Duration::from_secs(15))
                .user_agent(format!("User {}", config.github_username))
                .build()?,
            access_token: None,
            processed: vec![],
        })
    }

    /// Logs the bot in.
    ///
    /// Must be called before making any authenticated calls.
    pub async fn login(&mut self) -> Result<()> {
        debug!("Performing bot login");
        let form = {
            let mut form = HashMap::new();
            form.insert("grant_type", "password");
            form.insert("username", &self.config.username);
            form.insert("password", &self.config.password);
            form
        };
        let resp = self
            .reddit_client
            .post(&format!("{}/api/v1/access_token", BASE_URL))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&form)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Got status {} from login attempt", resp.status()));
        }
        let data = resp.json::<AccessTokenResponse>().await?;
        debug!("ATR from API: {:?}", data);
        self.reddit_client = build_client(&self.config, Some(data.token))?;

        Ok(())
    }

    /// Checks to see if a url matches a GH project without a license.
    async fn check_post(&self, url: &str) -> Result<bool> {
        let (org, repo) = match extract_gh_info(url) {
            Some(pair) => pair,
            None => return Err(anyhow!("Could not parse GitHub url at {}", url)),
        };
        {
            // check for valid project
            debug!("Checking for valid GH project");
            let url = format!("https://api.github.com/repos/{}/{}", org, repo);
            debug!("Checking {}", url);
            let resp = self.github_client.get(&url).send().await?;
            if !resp.status().is_success() {
                return Err(anyhow!(
                    "Invalid GH project '{}/{}' (got status {})",
                    org,
                    repo,
                    resp.status()
                ));
            } else {
                debug!("Project has a license");
            }
        }
        {
            // check for license
            let resp = self
                .github_client
                .get(&format!(
                    "https://api.github.com/repos/{}/{}/license",
                    org, repo
                ))
                .send()
                .await?;
            if !resp.status().is_success() {
                debug!(
                    "Got status {} from GitHub API for testing {}/{}",
                    resp.status(),
                    org,
                    repo
                );
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Responds
    async fn respond_to(&mut self, fullname: &str) -> Result<()> {
        debug!("Responding to post {}", fullname);
        let data = {
            let mut map = HashMap::new();
            map.insert("api_type", "json");
            map.insert("thing_id", fullname);
            map.insert("text", RESPONSE_TEXT);
            map
        };
        let resp = self
            .reddit_client
            .post(&format!("{}/api/comment", OAUTH_URL))
            .form(&data)
            .send()
            .await?;
        if !resp.status().is_success() {
            Err(anyhow!(
                "Got status {} from responding to post",
                resp.status()
            ))
        } else {
            Ok(())
        }
    }

    async fn delay(&self, subreddit: &str) {
        debug!(
            "No new posts in /r/{}, waiting {} seconds for checking again",
            subreddit, EMPTY_SUBREDDIT_DELAY
        );
        delay_for(time::Duration::from_secs(EMPTY_SUBREDDIT_DELAY)).await;
    }

    /// Single call to /r/{subreddit}/new and processing everything found.
    async fn watch_subreddit_once(
        &mut self,
        subreddit: &str,
        after: &Option<String>,
    ) -> Result<Option<String>> {
        debug!("Making request to see new from /r/{}", subreddit);
        let query = match after {
            Some(ref q) => vec![("raw_json", "1"), ("after", q)],
            None => vec![("raw_json", "1")],
        };
        let resp = self
            .reddit_client
            .get(&format!("{}/r/{}/new", OAUTH_URL, subreddit))
            .query(&query)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "Got status {} from listing endpoint",
                resp.status()
            ));
        }
        let data: Value = resp.json().await?;
        let postings = data["data"]["children"].as_array().unwrap();
        if postings.is_empty() {
            self.delay(subreddit).await;
            return Ok(after.to_owned());
        }
        for post_wrapper in postings {
            let post = &post_wrapper["data"];
            let fullname = post["name"].as_str().unwrap().to_owned();
            if self.processed.contains(&fullname) {
                continue;
            }
            self.processed.push(fullname.to_owned());
            if post["domain"].as_str().unwrap().starts_with("self.") {
                continue;
            }
            let url = post["url"].as_str().unwrap();
            debug!("Found link post to: {}", url);
            if url.contains("github.com") && self.check_post(url).await? {
                self.respond_to(&fullname).await?;
            }
        }
        if let Some(new_after) = data["data"]["after"].as_str() {
            debug!("After is now {}", new_after);
            Ok(Some(new_after.to_owned()))
        } else {
            self.delay(subreddit).await;
            Ok(after.to_owned())
        }
    }

    /// Watch a subreddit for all new posts.
    ///
    /// This function loops and does not return unless there's an error.
    pub async fn watch_subreddit(&mut self, subreddit: &str) -> Result<()> {
        let processed = {
            match fs::read_to_string(format!("processed-{}.json", subreddit)) {
                Ok(data) => match serde_json::from_str::<Vec<String>>(&data) {
                    Ok(data) => {
                        debug!("Loaded processed list with {} items", data.len());
                        data
                    }
                    Err(_) => vec![],
                },
                Err(_) => vec![],
            }
        };
        self.processed = processed;
        let mut after: Option<String> = None;
        loop {
            after = match self.watch_subreddit_once(subreddit, &after).await {
                Ok(a) => a,
                Err(e) => {
                    error!(
                        "Encountered error in processing loop for /r/{}: {}",
                        subreddit, e
                    );
                    after
                }
            };
            fs::write(
                format!("processed-{}.json", subreddit),
                serde_json::to_string(&self.processed)?,
            )?;
        }
    }
}
