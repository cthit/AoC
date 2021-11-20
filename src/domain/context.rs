use rocket::serde::Serialize;

use super::YearResponse;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context<T: Serialize> {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub current_nick: Option<String>,
	pub data: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsContext {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub aoc_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub github: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub year: Option<i32>,
	pub is_participating: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub owner: Option<OwnerContext>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerContext {
	pub years: Vec<YearResponse>,
}
