use rocket::serde::Serialize;

use super::{
	LeaderboardLanguagesResponse,
	LeaderboardResponse,
	LeaderboardSplitsResponse,
	YearResponse,
};

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardContext {
	pub year: i32,
	pub description: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub join_code: Option<String>,
	pub value_width: u8,
	pub leaderboard: Vec<LeaderboardPlacementContext>,
	pub next_update: String,
}

impl LeaderboardContext {
	pub fn format_next_update(secs_til_next_update: usize) -> String {
		format!(
			"{:0>2}:{:0>2}:{:0>2}",
			secs_til_next_update / (60 * 60),
			(secs_til_next_update / 60) % 60,
			secs_til_next_update % 60
		)
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardPlacementContext {
	pub nick: String,
	pub avatar_url: String,
	#[serde(skip_serializing_if = "is_none_or_empty")]
	pub github: Option<String>,
	pub value: String,
}

impl From<LeaderboardResponse> for LeaderboardPlacementContext {
	fn from(lr: LeaderboardResponse) -> Self {
		LeaderboardPlacementContext {
			nick: lr.nick,
			avatar_url: lr.avatar_url,
			github: lr.github,
			value: lr.score.to_string(),
		}
	}
}

impl From<LeaderboardSplitsResponse> for LeaderboardPlacementContext {
	fn from(lr: LeaderboardSplitsResponse) -> Self {
		LeaderboardPlacementContext {
			nick: lr.nick,
			avatar_url: lr.avatar_url,
			github: lr.github,
			value: if lr.split_average >= 60 * 60 * 24 {
				"--:--:--".to_string()
			} else {
				format!(
					"{:0>2}:{:0>2}:{:0>2}",
					(lr.split_average / (60 * 60)) % 24,
					(lr.split_average / 60) % 60,
					lr.split_average % 60
				)
			},
		}
	}
}

impl From<LeaderboardLanguagesResponse> for LeaderboardPlacementContext {
	fn from(lr: LeaderboardLanguagesResponse) -> Self {
		LeaderboardPlacementContext {
			nick: lr.nick,
			avatar_url: lr.avatar_url,
			github: lr.github,
			value: lr.languages.len().to_string(),
		}
	}
}

fn is_none_or_empty(s: &Option<String>) -> bool {
	s.is_none() || s.as_ref().unwrap().is_empty()
}
