use std::{collections::HashMap, env};

use lazy_static::lazy_static;
use reqwest::{Client, Error};
use rocket::{
	request::{FromRequest, Outcome},
	Request,
};

lazy_static! {
	static ref GITHUB_CLIENT_ID: String =
		env::var("GITHUB_CLIENT_ID").expect("Missing the GITHUB_CLIENT_ID environment variable.");
	static ref GITHUB_CLIENT_SECRET: String = env::var("GITHUB_CLIENT_SECRET")
		.expect("Missing the GITHUB_CLIENT_SECRET environment variable.");
	static ref GITHUB_CLIENT: GitHubClient = GitHubClient::new(
		GITHUB_CLIENT_ID.to_string(),
		GITHUB_CLIENT_SECRET.to_string()
	);
}

pub struct GitHubClient {
	client_id: String,
	client_secret: String,
	client: Client,
}

impl GitHubClient {
	pub fn new(client_id: String, client_secret: String) -> GitHubClient {
		GitHubClient {
			client_id,
			client_secret,
			client: Client::new(),
		}
	}

	pub async fn get_languages(&self, repo: &str) -> Result<HashMap<String, u64>, Error> {
		self.client
			.get(format!(
				"https://{}:{}@api.github.com/repos/{}/languages",
				self.client_id, self.client_secret, repo,
			))
			.header("User-Agent", "digIT-AoC-Server")
			.send()
			.await?
			.json()
			.await
	}
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'static GitHubClient {
	type Error = ();

	async fn from_request(_request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
		Outcome::Success(&GITHUB_CLIENT)
	}
}
