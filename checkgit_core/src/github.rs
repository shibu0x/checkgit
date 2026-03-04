use reqwest::{Client, StatusCode};
use serde::Deserialize;

use crate::{error::CheckGitError, models::GraphQLResponse};
use image::{DynamicImage, imageops::FilterType};

#[derive(Debug, Deserialize)]
pub struct GithubUserResponse {
    pub name: Option<String>,
    pub followers: u32,
    pub following: u32,
    pub avatar_url: String,
    pub bio: Option<String>,
    pub login: String,
    pub public_repos: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubRepoResponse {
    pub name: String,
    pub stargazers_count: u32,
}

pub struct GithubClient {
    client: Client,
    token: Option<String>,
}

impl GithubClient {
    pub fn new(token: Option<String>) -> Result<Self, CheckGitError> {
        let client = Client::builder().user_agent("checkgit").build()?;
        Ok(Self { client, token })
    }

    async fn send_request(&self, url: &str) -> Result<reqwest::Response, CheckGitError> {
        let response = self.client.get(url).send().await?;
        self.handle_status(response).await
    }

    async fn handle_status(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, CheckGitError> {
        let status = response.status();

        match status {
            StatusCode::NOT_FOUND => Err(CheckGitError::UserNotFound),
            StatusCode::FORBIDDEN => Err(CheckGitError::RateLimited),
            StatusCode::UNAUTHORIZED => Err(CheckGitError::Unauthorized),
            _ if status.is_server_error() => Err(CheckGitError::GithubServerError),
            _ if !status.is_success() => Err(CheckGitError::InvalidResponse),
            _ => Ok(response),
        }
    }

    pub async fn fetch_user(&self, username: &str) -> Result<GithubUserResponse, CheckGitError> {
        let url = format!("https://api.github.com/users/{}", username);
        let response = self.send_request(&url).await?;
        Ok(response.json::<GithubUserResponse>().await?)
    }

    pub async fn fetch_repos(
        &self,
        username: &str,
    ) -> Result<Vec<GithubRepoResponse>, CheckGitError> {
        let url = format!(
            "https://api.github.com/users/{}/repos?per_page=100&sort=stars&direction=desc",
            username
        );
        let response = self.send_request(&url).await?;
        Ok(response.json::<Vec<GithubRepoResponse>>().await?)
    }

    pub async fn fetch_avatar_image(
        &self,
        avatar_url: &str,
    ) -> Result<DynamicImage, CheckGitError> {
        let hi_res_url = if avatar_url.contains('?') {
            format!("{}&s=460", avatar_url)
        } else {
            format!("{}?s=460", avatar_url)
        };

        let response = self.client.get(&hi_res_url).send().await?;
        let bytes = response.bytes().await?;

        let img = image::load_from_memory(&bytes)
            .map_err(|e| CheckGitError::ImageError(e.to_string()))?;

        let size = img.width().min(img.height());
        let cropped = img.crop_imm(
            (img.width() - size) / 2,
            (img.height() - size) / 2,
            size,
            size,
        );

        let resized = cropped.resize(460, 460, FilterType::Lanczos3);

        let sharpened = resized.unsharpen(0.8, 2);

        Ok(sharpened)
    }

    pub async fn fetch_contributions(
        &self,
        username: &str,
    ) -> Result<Vec<Vec<u32>>, CheckGitError> {
        let token = self.token.as_ref().ok_or(CheckGitError::Unauthorized)?;

        let query = r#"
        query($login: String!) {
          user(login: $login) {
            contributionsCollection {
              contributionCalendar {
                weeks {
                  contributionDays {
                    contributionCount
                  }
                }
              }
            }
          }
        }
        "#;

        let body = serde_json::json!({
            "query": query,
            "variables": { "login": username }
        });

        let response = self
            .client
            .post("https://api.github.com/graphql")
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;

        let response = self.handle_status(response).await?;
        let text = response.text().await?;

        let parsed: GraphQLResponse =
            serde_json::from_str(&text).map_err(|_| CheckGitError::InvalidResponse)?;

        let weeks = parsed
            .data
            .unwrap()
            .user
            .contributions_collection
            .contribution_calendar
            .weeks;

        let mut matrix: Vec<Vec<u32>> = vec![Vec::new(); 7];

        for week in weeks {
            for (i, day) in week.contribution_days.into_iter().enumerate() {
                if i < 7 {
                    matrix[i].push(day.contribution_count);
                }
            }
        }

        Ok(matrix)
    }
}

pub fn calculate_total_stars(repos: &[GithubRepoResponse]) -> u32 {
    repos.iter().map(|r| r.stargazers_count).sum()
}