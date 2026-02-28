use reqwest::{Client, StatusCode};
use serde::Deserialize;

use crate::{error::CheckGitError, models::GraphQLResponse};

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

    pub async fn fetch_avatar_ascii(&self, avatar_url: &str) -> Result<String, CheckGitError> {
        let response = self.client.get(avatar_url).send().await?;
        let bytes = response.bytes().await?;

        let img = image::load_from_memory(&bytes)
            .map_err(|e| CheckGitError::ImageError(e.to_string()))?;

        let width = 60;
        let height = (width as f32 * 0.5) as u32;

        let img = img.resize(width, height, image::imageops::FilterType::Triangle);
        let grayscale = img.to_luma8();

        // Contrast normalization
        let mut min = 255u8;
        let mut max = 0u8;

        for pixel in grayscale.pixels() {
            let val = pixel[0];
            if val < min {
                min = val;
            }
            if val > max {
                max = val;
            }
        }

        let ascii_chars: Vec<char> =
            "@$B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'. "
                .chars()
                .collect();

        let mut output = String::new();

        for y in 0..grayscale.height() {
            for x in 0..grayscale.width() {
                let pixel = grayscale.get_pixel(x, y)[0];

                let normalized = if max > min {
                    (pixel.saturating_sub(min)) as f32 / (max - min) as f32
                } else {
                    0.0
                };

                let index = (normalized * (ascii_chars.len() - 1) as f32) as usize;
                output.push(ascii_chars[index]);
            }
            output.push('\n');
        }

        Ok(output)
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

        if parsed.errors.is_some() || parsed.data.is_none() {
            return Err(CheckGitError::InvalidResponse);
        }

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
