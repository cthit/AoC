use std::{collections::HashMap, env};

use lazy_static::lazy_static;
use reqwest::{Client, Error};
use rocket::{
	request::{FromRequest, Outcome},
	Request,
};
use serde::{Deserialize, Serialize};

lazy_static! {
	static ref AOC_SESSION: String =
		env::var("AOC_SESSION").expect("Missing the AOC_SESSION environment variable.");
	static ref AOC_CLIENT: AocClient = AocClient::new(AOC_SESSION.to_string());
}

pub struct AocClient {
	session: String,
	client: Client,
}

impl AocClient {
	pub fn new(session: String) -> AocClient {
		AocClient {
			session,
			client: Client::new(),
		}
	}

	pub async fn get_leaderboard(
		&self,
		year: i32,
		leaderboard: &str,
	) -> Result<Leaderboard, Error> {
		self.client
			.get(format!(
				"https://adventofcode.com/{}/leaderboard/private/view/{}.json",
				year, leaderboard
			))
			.header("Cookie", format!("session={}", self.session))
			.send()
			.await?
			.json()
			.await
	}
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'static AocClient {
	type Error = ();

	async fn from_request(_request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
		Outcome::Success(&AOC_CLIENT)
	}
}

#[derive(Deserialize, Serialize)]
pub struct Leaderboard {
	pub event: String,
	pub owner_id: String,
	pub members: HashMap<String, Member>,
}

#[derive(Deserialize, Serialize)]
pub struct Member {
	pub stars: u8,
	#[serde(default)]
	pub name: Option<String>,
	#[serde(deserialize_with = "string_or_u64")]
	pub last_star_ts: u64,
	pub completion_day_level: HashMap<String, Day>,
	pub id: String,
	pub local_score: u16,
	pub global_score: u16,
}

fn string_or_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let sou = StringOrU64::deserialize(deserializer)?;
	match sou {
		StringOrU64::String(s) => s.parse::<u64>().map_err(serde::de::Error::custom),
		StringOrU64::U64(u) => Ok(u),
	}
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrU64 {
	U64(u64),
	String(String),
}

#[derive(Deserialize, Serialize)]
pub struct Day {
	#[serde(rename = "1", default, deserialize_with = "from_get_star_ts")]
	pub first_star_ts: Option<u64>,
	#[serde(rename = "2", default, deserialize_with = "from_get_star_ts")]
	pub second_star_ts: Option<u64>,
}

fn from_get_star_ts<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let s: Option<GetStarTs> = Option::deserialize(deserializer)?;
	Ok(s.map(|gst| gst.get_star_ts))
}

#[derive(Deserialize)]
struct GetStarTs {
	get_star_ts: u64,
}
