use std::{cmp::Reverse, time::SystemTime};

use diesel::{expression_methods::ExpressionMethods, query_dsl::QueryDsl, RunQueryDsl};
use rocket::{http::Status, serde::Serialize};

use crate::{
	aoc_client::AocClient,
	db::{participants, users, years, DbConn, Participant, User, Year},
	gamma::GammaClient,
	github_client::GitHubClient,
};

pub async fn get_leaderboard(
	year: i32,
	conn: &DbConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<Vec<LeaderboardResponse>, Status> {
	let year_db: Year = conn
		.run(move |c| years::table.filter(years::columns::year.eq(year)).first(c))
		.await
		.map_err(|err| {
			println!("Could not find in db: {:?}", err);
			Status::NotFound
		})?;

	let leaderboard = aoc_client
		.get_leaderboard(year, &year_db.leaderboard)
		.await
		.map_err(|err| {
			println!("Could not find on AoC: {:?}", err);
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
		.map_err(|_| Status::InternalServerError)?;

	let mut response = futures::future::join_all(
		participants
			.drain(..)
			.map(|(p, u)| LeaderboardResponse {
				cid: u.cid.clone(),
				nick: String::new(),
				avatar_url: String::new(),
				github: p.github,
				score: leaderboard.members[&u.aoc_id].local_score,
			})
			.map(async move |mut lr| {
				let user = gamma_client.get_user(&lr.cid).await.unwrap();
				lr.nick.push_str(&user.nick);
				lr.avatar_url.push_str(&user.avatar_url);
				lr
			}),
	)
	.await;
	response.sort_by_key(|lr| Reverse(lr.score));
	Ok(response)
}

pub async fn get_leaderboard_splits(
	year: i32,
	conn: &DbConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<Vec<LeaderboardSplitsResponse>, Status> {
	let year_db: Year = conn
		.run(move |c| years::table.filter(years::columns::year.eq(year)).first(c))
		.await
		.map_err(|err| {
			println!("Could not find in db: {:?}", err);
			Status::NotFound
		})?;

	let leaderboard = aoc_client
		.get_leaderboard(year, &year_db.leaderboard)
		.await
		.map_err(|err| {
			println!("Could not find on AoC: {:?}", err);
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
		.map_err(|_| Status::InternalServerError)?;

	let mut response = futures::future::join_all(
		participants
			.drain(..)
			.map(|(p, u)| {
				const ONE_DAY: u64 = 24 * 60 * 60 * 1000;
				let current = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
					Ok(dur) => dur.as_millis() as u64,
					Err(_) => panic!("SystemTime before UNIX EPOCH!"),
				};
				let mut split_count = 0;
				LeaderboardSplitsResponse {
					cid: u.cid.clone(),
					nick: String::new(),
					avatar_url: String::new(),
					github: p.github,
					split_average: leaderboard.members[&u.aoc_id]
						.completion_day_level
						.values()
						.filter_map(|d| match (d.first_star_ts, d.second_star_ts) {
							(Some(first), Some(second)) => {
								split_count += 1;
								Some((second - first).min(ONE_DAY))
							}
							(Some(first), None) => {
								split_count += 1;
								Some((current - first).min(ONE_DAY))
							}
							_ => None,
						})
						.sum::<u64>()
						.checked_div(split_count)
						.unwrap_or(0),
				}
			})
			.map(async move |mut lr| {
				let user = gamma_client.get_user(&lr.cid).await.unwrap();
				lr.nick.push_str(&user.nick);
				lr.avatar_url.push_str(&user.avatar_url);
				lr
			}),
	)
	.await;
	response.sort_by_key(|lr| lr.split_average);
	Ok(response)
}

pub async fn get_leaderboard_languages(
	year: i32,
	conn: &DbConn,
	gamma_client: &GammaClient,
	github_client: &GitHubClient,
) -> Result<Vec<LeaderboardLanguagesResponse>, Status> {
	let mut participants: Vec<(Participant, User)> = conn
		.run(move |c| {
			participants::table
				.inner_join(users::table)
				.filter(participants::columns::year.eq(year))
				.filter(participants::columns::github.is_not_null())
				.load(c)
		})
		.await
		.map_err(|_| Status::InternalServerError)?;

	let mut response =
		futures::future::join_all(participants.drain(..).map(async move |(p, u)| {
			let user = gamma_client.get_user(&u.cid).await.unwrap();
			let languages = github_client
				.get_languages(p.github.as_ref().unwrap())
				.await
				.unwrap();
			LeaderboardLanguagesResponse {
				cid: u.cid.clone(),
				nick: user.nick,
				avatar_url: user.avatar_url,
				github: p.github,
				languages: languages.into_keys().collect(),
			}
		}))
		.await;
	response.sort_by_key(|lr| Reverse(lr.languages.len()));
	Ok(response)
}

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
