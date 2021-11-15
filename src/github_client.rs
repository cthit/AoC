use std::collections::HashMap;

use reqwest::{Client, Error};

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
