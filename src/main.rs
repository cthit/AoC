#![feature(never_type, proc_macro_hygiene, decl_macro, async_closure)]

mod aoc_client;
mod db;
mod domain;
mod gamma;
mod github_client;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;

use std::{cmp::Reverse, time::SystemTime};

use aoc_client::AocClient;
use db::DbConn;
use diesel::{expression_methods::ExpressionMethods, query_dsl::QueryDsl, RunQueryDsl};
use domain::{
	AocIdRequest,
	AocIdResponse,
	LeaderboardLanguagesResponse,
	LeaderboardResponse,
	LeaderboardSplitsResponse,
	ParticipateRequest,
	ParticipateResponse,
	YearDeleteRequest,
	YearRequest,
	YearResponse,
};
use gamma::GammaClient;
use github_client::GitHubClient;
use rocket::{
	fairing::AdHoc,
	http::{uri::Origin, Cookie, CookieJar, Status},
	response::Redirect,
	serde::json::Json,
	Build,
	Rocket,
};

#[get("/login?<back>")]
async fn login(
	back: Option<String>,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Redirect {
	if let Some(access_cookie) = cookies.get(GammaClient::cookie()) {
		if gamma_client.get_me(access_cookie.value()).await.is_ok() {
			return Redirect::to(back.unwrap_or_else(|| "/".to_string()));
		}
	}
	Redirect::to(gamma_client.authorize_url(back.unwrap_or_else(|| "/".to_string())))
}

#[get("/aoc-id.json")]
async fn get_aoc_id_json(
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Json<AocIdResponse>, Status> {
	use db::{users, User};

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	let user_db: User = conn
		.run(move |c| {
			users::table
				.filter(users::columns::cid.eq(user.cid))
				.first(c)
		})
		.await
		.map_err(|err| match err {
			diesel::result::Error::NotFound => Status::NotFound,
			_ => Status::InternalServerError,
		})?;
	Ok(Json(AocIdResponse {
		aoc_id: user_db.aoc_id,
	}))
}

#[post("/aoc-id.json", data = "<data>")]
async fn post_aoc_id_json(
	data: Json<AocIdRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	use db::{users, User};

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	conn.run(move |c| {
		diesel::insert_into(users::table)
			.values(User {
				cid: user.cid,
				aoc_id: data.aoc_id.clone(),
			})
			.on_conflict(users::columns::cid)
			.do_update()
			.set(users::columns::aoc_id.eq(data.aoc_id.clone()))
			.execute(c)
	})
	.await
	.map_err(|_| Status::InternalServerError)?;
	Ok(Status::Ok)
}

#[get("/years.json")]
async fn get_years_json(conn: DbConn) -> Result<Json<Vec<YearResponse>>, Status> {
	use db::{years, Year};

	let mut years_db: Vec<Year> = conn
		.run(move |c| years::table.load(c))
		.await
		.map_err(|_| Status::InternalServerError)?;
	Ok(Json(
		years_db
			.drain(..)
			.map(|y| YearResponse {
				year: y.year,
				leaderboard: y.leaderboard,
			})
			.collect(),
	))
}

#[post("/years.json", data = "<data>")]
async fn post_years_json(
	data: Json<YearRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	use db::{years, Year};

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	if !user
		.groups
		.ok_or(Status::Forbidden)?
		.iter()
		.any(|g| g.name == GammaClient::owner_group())
	{
		return Err(Status::Forbidden);
	}
	conn.run(move |c| {
		diesel::insert_into(years::table)
			.values(Year {
				year: data.year,
				leaderboard: data.leaderboard.clone(),
			})
			.on_conflict(years::columns::year)
			.do_update()
			.set(years::columns::leaderboard.eq(data.leaderboard.clone()))
			.execute(c)
	})
	.await
	.map_err(|_| Status::InternalServerError)?;
	Ok(Status::Ok)
}

#[delete("/years.json", data = "<data>")]
async fn delete_years_json(
	data: Json<YearDeleteRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	use db::years;

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	if !user
		.groups
		.ok_or(Status::Forbidden)?
		.iter()
		.any(|g| g.name == GammaClient::owner_group())
	{
		return Err(Status::Forbidden);
	}
	let rows_deleted = conn
		.run(move |c| {
			diesel::delete(years::table)
				.filter(years::columns::year.eq(data.year))
				.execute(c)
		})
		.await
		.map_err(|_| Status::InternalServerError)?;
	if rows_deleted == 1 {
		Ok(Status::Ok)
	} else {
		Err(Status::NotFound)
	}
}

#[get("/participate.json")]
async fn get_participate_json(
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Json<Vec<ParticipateResponse>>, Status> {
	use db::{participants, Participant};

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	let mut participant: Vec<Participant> = conn
		.run(move |c| {
			participants::table
				.filter(participants::columns::cid.eq(user.cid))
				.load(c)
		})
		.await
		.map_err(|err| match err {
			diesel::result::Error::NotFound => Status::NotFound,
			_ => Status::InternalServerError,
		})?;
	Ok(Json(
		participant
			.drain(..)
			.map(|p| ParticipateResponse {
				year: p.year,
				github: p.github,
			})
			.collect(),
	))
}

#[post("/participate.json", data = "<data>")]
async fn post_participate_json(
	data: Json<ParticipateRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	use db::{participants, Participant};

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	conn.run(move |c| {
		diesel::insert_into(participants::table)
			.values(Participant {
				cid: user.cid,
				year: data.year,
				github: data.github.clone(),
			})
			.on_conflict((participants::columns::cid, participants::columns::year))
			.do_update()
			.set(participants::columns::github.eq(data.github.clone()))
			.execute(c)
	})
	.await
	.map_err(|_| Status::InternalServerError)?;
	Ok(Status::Ok)
}

#[delete("/participate.json", data = "<data>")]
async fn delete_participate_json(
	data: Json<ParticipateRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	use db::participants;

	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;
	let rows_deleted = conn
		.run(move |c| {
			diesel::delete(participants::table)
				.filter(participants::columns::cid.eq(user.cid))
				.filter(participants::columns::year.eq(data.year))
				.execute(c)
		})
		.await
		.map_err(|_| Status::InternalServerError)?;
	if rows_deleted == 1 {
		Ok(Status::Ok)
	} else {
		Err(Status::NotFound)
	}
}

#[get("/leaderboard/<year>")]
async fn get_leaderboard_year_json(
	mut year: String,
	conn: DbConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<Json<Vec<LeaderboardResponse>>, Status> {
	use db::{participants, users, years, Participant, User, Year};

	if !year.ends_with(".json") {
		return Err(Status::NotFound);
	}

	year.truncate(year.len() - 5);
	let year: i32 = year.parse().map_err(|err| {
		println!("Could not parse: {:?}", err);
		Status::NotFound
	})?;

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
	Ok(Json(response))
}

#[get("/leaderboard/<year>/splits.json")]
async fn get_leaderboard_year_splits_json(
	year: i32,
	conn: DbConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<Json<Vec<LeaderboardSplitsResponse>>, Status> {
	use db::{participants, users, years, Participant, User, Year};

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
	Ok(Json(response))
}

#[get("/leaderboard/<year>/languages.json")]
async fn get_leaderboard_year_languages_json(
	year: i32,
	conn: DbConn,
	gamma_client: &GammaClient,
	github_client: &GitHubClient,
) -> Result<Json<Vec<LeaderboardLanguagesResponse>>, Status> {
	use db::{participants, users, Participant, User};

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
	Ok(Json(response))
}

#[get("/callback?<code>&<state>")]
async fn callback(
	code: String,
	state: Option<String>,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Redirect, Status> {
	let access_token = gamma_client
		.get_token(code)
		.await
		.map_err(|_| Status::Unauthorized)?;
	gamma_client
		.get_me(&access_token)
		.await
		.map_err(|_| Status::Unauthorized)?;
	cookies.add(
		Cookie::build(GammaClient::cookie(), access_token)
			.path("/")
			.finish(),
	);
	Ok(Redirect::to(
		state
			.map(|s| Origin::parse_owned(s).ok())
			.flatten()
			.unwrap_or(uri!(index)),
	))
}

#[get("/callback?<error>&<error_description>", rank = 2)]
async fn callback_error(error: String, error_description: Option<String>) -> String {
	match error_description {
		Some(desc) => format!("{}\n{}", error, desc),
		None => error,
	}
}

#[get("/")]
fn index() -> &'static str {
	"Hello, world!"
}

#[launch]
fn rocket() -> Rocket<Build> {
	rocket::build()
		.attach(DbConn::fairing())
		.attach(AdHoc::on_liftoff("Initialize the AoC database", |rocket| {
			Box::pin(async move {
				if let Err(e) = db::initialize(rocket).await {
					eprintln!(
						"Failed to initialize the database connection. ({})\nShutting down...",
						e
					);
					rocket.shutdown().notify();
				}
			})
		}))
		.mount("/", routes![
			index,
			login,
			callback,
			callback_error,
			get_aoc_id_json,
			post_aoc_id_json,
			get_years_json,
			post_years_json,
			delete_years_json,
			get_participate_json,
			post_participate_json,
			delete_participate_json,
			get_leaderboard_year_json,
			get_leaderboard_year_splits_json,
			get_leaderboard_year_languages_json,
		])
}
