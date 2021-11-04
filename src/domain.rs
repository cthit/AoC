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
