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

use aoc_client::AocClient;
use db::DbConn;
use domain::{
	delete_participation,
	delete_year,
	get_aoc_id,
	get_leaderboard,
	get_leaderboard_languages,
	get_leaderboard_splits,
	get_participations,
	get_years,
	set_aoc_id,
	set_participation,
	set_year,
	AocIdRequest,
	AocIdResponse,
	LeaderboardLanguagesResponse,
	LeaderboardResponse,
	LeaderboardSplitsResponse,
	ParticipateDeleteRequest,
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
	get_aoc_id(&conn, cookies, gamma_client).await.map(Json)
}

#[post("/aoc-id.json", data = "<data>")]
async fn post_aoc_id_json(
	data: Json<AocIdRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	set_aoc_id(data.aoc_id.clone(), &conn, cookies, gamma_client)
		.await
		.map(|_| Status::Ok)
}

#[get("/years.json")]
async fn get_years_json(conn: DbConn) -> Result<Json<Vec<YearResponse>>, Status> {
	get_years(&conn).await.map(Json)
}

#[post("/years.json", data = "<data>")]
async fn post_years_json(
	data: Json<YearRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	set_year(data.0, &conn, cookies, gamma_client)
		.await
		.map(|_| Status::Ok)
}

#[delete("/years.json", data = "<data>")]
async fn delete_years_json(
	data: Json<YearDeleteRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	delete_year(data.0, &conn, cookies, gamma_client)
		.await
		.map(|_| Status::Ok)
}

#[get("/participate.json")]
async fn get_participate_json(
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Json<Vec<ParticipateResponse>>, Status> {
	get_participations(&conn, cookies, gamma_client)
		.await
		.map(Json)
}

#[post("/participate.json", data = "<data>")]
async fn post_participate_json(
	data: Json<ParticipateRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	set_participation(data.0, &conn, cookies, gamma_client)
		.await
		.map(|_| Status::Ok)
}

#[delete("/participate.json", data = "<data>")]
async fn delete_participate_json(
	data: Json<ParticipateDeleteRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Status, Status> {
	delete_participation(data.0, &conn, cookies, gamma_client)
		.await
		.map(|_| Status::Ok)
}

#[get("/leaderboard/<year>")]
async fn get_leaderboard_year_json(
	mut year: String,
	conn: DbConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<Json<Vec<LeaderboardResponse>>, Status> {
	if !year.ends_with(".json") {
		return Err(Status::NotFound);
	}

	year.truncate(year.len() - 5);
	let year: i32 = year.parse().map_err(|err| {
		println!("Could not parse: {:?}", err);
		Status::NotFound
	})?;

	get_leaderboard(year, &conn, aoc_client, gamma_client)
		.await
		.map(Json)
}

#[get("/leaderboard/<year>/splits.json")]
async fn get_leaderboard_year_splits_json(
	year: i32,
	conn: DbConn,
	aoc_client: &AocClient,
	gamma_client: &GammaClient,
) -> Result<Json<Vec<LeaderboardSplitsResponse>>, Status> {
	get_leaderboard_splits(year, &conn, aoc_client, gamma_client)
		.await
		.map(Json)
}

#[get("/leaderboard/<year>/languages.json")]
async fn get_leaderboard_year_languages_json(
	year: i32,
	conn: DbConn,
	gamma_client: &GammaClient,
	github_client: &GitHubClient,
) -> Result<Json<Vec<LeaderboardLanguagesResponse>>, Status> {
	get_leaderboard_languages(year, &conn, gamma_client, github_client)
		.await
		.map(Json)
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
