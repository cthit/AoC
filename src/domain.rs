use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AocIdRequest {
	pub aoc_id: String,
}

pub type AocIdResponse = AocIdRequest;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YearRequest {
	pub year: i32,
	pub leaderboard: String,
}

pub type YearResponse = YearRequest;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YearDeleteRequest {
	pub year: i32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipateRequest {
	pub year: i32,
	#[serde(default)]
	pub github: Option<String>,
}

pub type ParticipateResponse = ParticipateRequest;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardResponse {
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub github: Option<String>,
	pub score: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardSplitsResponse {
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub github: Option<String>,
	pub split_average: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardLanguagesResponse {
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub github: Option<String>,
	pub languages: Vec<String>,
}
