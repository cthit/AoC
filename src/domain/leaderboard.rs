use std::{cmp::Reverse, collections::HashMap};

use diesel::{expression_methods::ExpressionMethods, query_dsl::QueryDsl, RunQueryDsl};
use lazy_static::lazy_static;
use r2d2_redis::redis::Commands;
use rocket::{
	http::Status,
	serde::{
		json::{serde_json, Json},
		Deserialize,
		DeserializeOwned,
		Serialize,
	},
};
use rocket_dyn_templates::Template;

use crate::{
	aoc_client::AocClient,
	db::{participants, users, years, DbConn, Participant, User, Year},
	gamma::GammaClient,
	github_client::GitHubClient,
	redis::RedisConn,
};

lazy_static! {
	static ref LEADERBOARD_CACHE_TIME: usize = std::env::var("LEADERBOARD_CACHE_TIME")
		.map(|s| s.parse().unwrap())
		.unwrap_or(60);
	static ref LEADERBOARD_SPLITS_CACHE_TIME: usize = std::env::var("LEADERBOARD_CACHE_TIME")
		.map(|s| s.parse().unwrap())
		.unwrap_or(60);
	static ref LEADERBOARD_LANGUAGES_CACHE_TIME: usize = std::env::var("LEADERBOARD_CACHE_TIME")
		.map(|s| s.parse().unwrap())
		.unwrap_or(60);
}

async fn fetch_from_cache<T: DeserializeOwned>(
	redis: &RedisConn,
	key: &str,
) -> Result<Option<(Vec<T>, usize)>, Status> {
	let key_copy = key.to_owned();
	let cache_result = redis
		.run(move |c| {
			c.get::<_, Option<String>>(key_copy.clone())
				.map(|possible_cache| {
					if let Some(cached) = possible_cache {
						c.ttl(key_copy).map(|ttl| Some((cached, ttl)))
					} else {
						Ok(None)
					}
				})
		})
		.await
		.map_err(|e| {
			println!("Redis error: {}", e);
			Status::InternalServerError
		})?
		.map_err(|e| {
			println!("Redis error: {}", e);
			Status::InternalServerError
		})?;
	if let Some((cached, ttl)) = cache_result {
		if let Ok(cached) = serde_json::from_str::<Vec<T>>(&cached).map_err(|e| {
			println!("Malformatted redis value: {}", e);
		}) {
			return Ok(Some((cached, ttl)));
		}
	}

	Ok(None)
}

async fn cache_leaderboard<T: Serialize>(
	redis: &RedisConn,
	key: String,
	response: &[T],
	time: usize,
) {
	let cache = serde_json::to_string(response).unwrap();
	let result = redis
		.run(move |c| c.set_ex::<_, _, ()>(&key, cache, time))
		.await;

	if let Err(err) = result {
		println!("Could not cache leaderboard: {:?}", err);
	}
}

pub async fn get_leaderboard(
	year: i32,
	conn: &DbConn,
	redis: &RedisConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<(Vec<LeaderboardResponse>, usize), Status> {
	let redis_key = format!("leaderboard_{}", year);

	if let Some((cached, ttl)) = fetch_from_cache(redis, &redis_key).await? {
		return Ok((cached, ttl));
	}

	let year_db: Year = conn
		.run(move |c| years::table.filter(years::columns::year.eq(year)).first(c))
		.await
		.map_err(|_| Status::NotFound)?;

	let leaderboard = aoc_client
		.get_leaderboard(year, year_db.leaderboard_id())
		.await
		.map_err(|e| {
			println!(
				"Could not find AoC leaderboard (year {}, id \"{}\") when loading leaderboard \
				 ({}:{})\n\t{:?}",
				year,
				year_db.leaderboard_id(),
				file!(),
				line!(),
				e
			);
			Status::InternalServerError
		})?;

	let mut participants: Vec<(Participant, User)> = conn
		.run(move |c| {
			participants::table
				.inner_join(users::table)
				.filter(participants::columns::year.eq(year))
				.load(c)
		})
		.await
		.map_err(|e| {
			println!(
				"Could not fetch from database when loading leaderboard ({}:{})\n\t{:?}",
				file!(),
				line!(),
				e
			);
			Status::InternalServerError
		})?;

	let mut response: Vec<Result<_, ()>> = futures::future::join_all(
		participants
			.drain(..)
			.filter_map(|(p, u)| {
				leaderboard
					.members
					.get(&u.aoc_id)
					.map(|m| LeaderboardResponse {
						cid: u.cid.clone(),
						nick: String::new(),
						avatar_url: String::new(),
						github: p.github,
						score: m.local_score,
					})
			})
			.map(async move |mut lr| {
				let user = gamma_client.get_user(&lr.cid).await.map_err(|e| {
					println!(
						"Could not get user {} when loading leaderboard ({}:{})\n\t{:?}",
						lr.cid,
						file!(),
						line!(),
						e
					);
				})?;
				lr.nick.push_str(&user.nick);
				lr.avatar_url.push_str(&user.avatar_url);
				Ok(lr)
			}),
	)
	.await;
	let mut response: Vec<_> = response.drain(..).filter_map(|r| r.ok()).collect();
	response.sort_by_key(|lr| Reverse(lr.score));

	cache_leaderboard(redis, redis_key, &response, *LEADERBOARD_CACHE_TIME).await;

	Ok((response, *LEADERBOARD_CACHE_TIME))
}

pub async fn get_leaderboard_splits(
	year: i32,
	conn: &DbConn,
	redis: &RedisConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<(Vec<LeaderboardSplitsResponse>, usize), Status> {
	let redis_key = format!("leaderboard_splits_{}", year);

	if let Some((cached, ttl)) = fetch_from_cache(redis, &redis_key).await? {
		return Ok((cached, ttl));
	}

	let year_db: Year = conn
		.run(move |c| years::table.filter(years::columns::year.eq(year)).first(c))
		.await
		.map_err(|_| Status::NotFound)?;

	let leaderboard = aoc_client
		.get_leaderboard(year, year_db.leaderboard_id())
		.await
		.map_err(|e| {
			println!(
				"Could not find AoC leaderboard (year {}, id \"{}\") when loading leaderboard \
				 ({}:{})\n\t{:?}",
				year,
				year_db.leaderboard_id(),
				file!(),
				line!(),
				e
			);
			Status::InternalServerError
		})?;

	let participants: Vec<(Participant, User)> = conn
		.run(move |c| {
			participants::table
				.inner_join(users::table)
				.filter(participants::columns::year.eq(year))
				.load(c)
		})
		.await
		.map_err(|e| {
			println!(
				"Could not fetch from database when loading leaderboard ({}:{})\n\t{:?}",
				file!(),
				line!(),
				e
			);
			Status::InternalServerError
		})?;

	let unprocessed_members: Vec<_> = participants
		.into_iter()
		.filter_map(|(p, u)| leaderboard.members.get(&u.aoc_id).map(|m| (m, p, u)))
		.collect();

	let mut members: Vec<Result<_, ()>> = futures::future::join_all(
		unprocessed_members
			.iter()
			.map(|(_, p, u)| LeaderboardSplitsResponse {
				cid: u.cid.to_owned(),
				nick: String::new(),
				avatar_url: String::new(),
				github: p.github.to_owned(),
				score: 0,
			})
			.map(async move |mut lr| {
				let user = gamma_client.get_user(&lr.cid).await.map_err(|e| {
					println!(
						"Could not get user {} when loading leaderboard ({}:{})\n\t{:?}",
						lr.cid,
						file!(),
						line!(),
						e
					);
				})?;
				lr.nick.push_str(&user.nick);
				lr.avatar_url.push_str(&user.avatar_url);
				Ok(lr)
			}),
	)
	.await;
	let mut members: HashMap<_, _> = members
		.drain(..)
		.filter_map(|r| r.ok().map(|m| (m.cid.to_owned(), m)))
		.collect();

	let total_members = members.len() as u16;
	let mut day_vec = Vec::new();
	for day in 1..=25 {
		let day_str = day.to_string();

		day_vec.extend(unprocessed_members.iter().filter_map(|(m, _, u)| {
			m.completion_day_level
				.get(&day_str)
				.map(|d| match (d.first_star_ts, d.second_star_ts) {
					(Some(f), Some(s)) => Some((&u.cid, s - f)),
					_ => None,
				})
				.flatten()
		}));
		day_vec.sort_by_key(|&(_, split)| split);

		for (i, (cid, _)) in day_vec.iter().enumerate() {
			if let Some(m) = members.get_mut(*cid) {
				m.score += total_members - i as u16;
			}
		}

		day_vec.clear();
	}

	let mut response: Vec<_> = members.drain().map(|(_, v)| v).collect();
	response.sort_by_key(|lr| Reverse(lr.score));

	cache_leaderboard(redis, redis_key, &response, *LEADERBOARD_SPLITS_CACHE_TIME).await;

	Ok((response, *LEADERBOARD_SPLITS_CACHE_TIME))
}

pub async fn get_leaderboard_languages(
	year: i32,
	conn: &DbConn,
	redis: &RedisConn,
	gamma_client: &GammaClient,
	github_client: &GitHubClient,
) -> Result<(Vec<LeaderboardLanguagesResponse>, usize), Status> {
	let redis_key = format!("leaderboard_languages_{}", year);

	if let Some((cached, ttl)) = fetch_from_cache(redis, &redis_key).await? {
		return Ok((cached, ttl));
	}

	let mut participants: Vec<(Participant, User)> = conn
		.run(move |c| {
			participants::table
				.inner_join(users::table)
				.filter(participants::columns::year.eq(year))
				.filter(participants::columns::github.is_not_null())
				.filter(participants::columns::github.ne(""))
				.load(c)
		})
		.await
		.map_err(|e| {
			println!(
				"Could not fetch from database when loading leaderboard ({}:{})\n\t{:?}",
				file!(),
				line!(),
				e
			);
			Status::InternalServerError
		})?;

	let mut response: Vec<Result<_, ()>> =
		futures::future::join_all(participants.drain(..).map(async move |(p, u)| {
			let user = gamma_client.get_user(&u.cid).await.map_err(|e| {
				println!(
					"Could not get user {} when loading leaderboard ({}:{})\n\t{:?}",
					u.cid,
					file!(),
					line!(),
					e
				);
			})?;
			let github = p.github.as_ref().unwrap();
			let languages = github_client.get_languages(github).await.map_err(|e| {
				println!(
					"Could not get repo {:?} when loading leaderboard ({}:{})\n\t{:?}",
					github,
					file!(),
					line!(),
					e
				);
			})?;
			Ok(LeaderboardLanguagesResponse {
				cid: u.cid.clone(),
				nick: user.nick,
				avatar_url: user.avatar_url,
				github: p.github,
				languages: languages.into_keys().collect(),
			})
		}))
		.await;
	let mut response: Vec<_> = response.drain(..).filter_map(|r| r.ok()).collect();
	response.sort_by_key(|lr| Reverse(lr.languages.len()));

	cache_leaderboard(
		redis,
		redis_key,
		&response,
		*LEADERBOARD_LANGUAGES_CACHE_TIME,
	)
	.await;

	Ok((response, *LEADERBOARD_LANGUAGES_CACHE_TIME))
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardResponse {
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub github: Option<String>,
	pub score: u16,
}

#[derive(Responder)]
pub enum JsonOrTemplateLeaderboard {
	Json(Json<Vec<LeaderboardResponse>>),
	Template(Template),
}

impl JsonOrTemplateLeaderboard {
	pub fn json(leaderboard: Vec<LeaderboardResponse>) -> Self {
		JsonOrTemplateLeaderboard::Json(Json(leaderboard))
	}
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardSplitsResponse {
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub github: Option<String>,
	pub score: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardLanguagesResponse {
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub github: Option<String>,
	pub languages: Vec<String>,
}
