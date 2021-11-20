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
	Context,
	LeaderboardLanguagesResponse,
	LeaderboardResponse,
	LeaderboardSplitsResponse,
	OwnerContext,
	ParticipateDeleteRequest,
	ParticipateRequest,
	ParticipateResponse,
	SettingsContext,
	YearDeleteRequest,
	YearRequest,
	YearResponse,
};
use gamma::GammaClient;
use github_client::GitHubClient;
use rocket::{
	fairing::AdHoc,
	form::Form,
	fs::FileServer,
	http::{uri::Origin, Cookie, CookieJar, Status},
	response::Redirect,
	serde::{json::Json, Serialize},
	Build,
	Rocket,
};
use rocket_dyn_templates::Template;

async fn create_base_context<T: Serialize>(
	data: T,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Context<T> {
	if let Some(access_cookie) = cookies.get(GammaClient::cookie()) {
		if let Ok(user) = gamma_client.get_me(access_cookie.value()).await {
			return Context {
				current_nick: Some(user.nick),
				data,
			};
		}
	}

	Context {
		current_nick: None,
		data,
	}
}

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

#[post("/aoc-id", data = "<data>")]
async fn post_aoc_id(
	data: Form<AocIdRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Redirect, Status> {
	set_aoc_id(data.aoc_id.clone(), &conn, cookies, gamma_client).await?;
	Ok(Redirect::to(uri!(settings)))
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

#[post("/years", data = "<data>")]
async fn post_years(
	data: Form<YearRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Redirect, Status> {
	set_year(data.into_inner(), &conn, cookies, gamma_client).await?;
	Ok(Redirect::to(uri!(settings)))
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

#[post("/years-delete", data = "<data>")]
async fn delete_years(
	data: Form<YearDeleteRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Redirect, Status> {
	delete_year(data.into_inner(), &conn, cookies, gamma_client).await?;
	Ok(Redirect::to(uri!(settings)))
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

#[post("/participate", data = "<data>")]
async fn post_participate(
	data: Form<ParticipateRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Redirect, Status> {
	set_participation(data.into_inner(), &conn, cookies, gamma_client).await?;
	Ok(Redirect::to(uri!(settings)))
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

#[post("/participate-delete", data = "<data>")]
async fn delete_participate(
	data: Form<ParticipateDeleteRequest>,
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Redirect, Status> {
	delete_participation(data.into_inner(), &conn, cookies, gamma_client).await?;
	Ok(Redirect::to(uri!(settings)))
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
async fn index(cookies: &CookieJar<'_>, gamma_client: &GammaClient) -> Template {
	let context = create_base_context((), cookies, gamma_client).await;
	Template::render("index", context)
}

#[get("/about")]
async fn about(cookies: &CookieJar<'_>, gamma_client: &GammaClient) -> Template {
	let context = create_base_context((), cookies, gamma_client).await;
	Template::render("about", context)
}

#[get("/settings")]
async fn settings(
	conn: DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Template, Status> {
	let aoc_id = get_aoc_id(&conn, cookies, gamma_client)
		.await
		.map(|aoc_id| Some(aoc_id.aoc_id))
		.or_else(|e| {
			if e.code == Status::NotFound.code {
				Ok(None)
			} else {
				Err(e)
			}
		})?;
	let year = get_years(&conn).await?.into_iter().map(|y| y.year).max();
	let (github, is_participating) = match year {
		Some(year) => get_participations(&conn, cookies, gamma_client)
			.await?
			.drain(..)
			.find(|p| p.year == year)
			.map_or_else(|| (None, false), |p| (p.github, true)),
		None => (None, false),
	};
	let owner = if let Some(access_cookie) = cookies.get(GammaClient::cookie()) {
		if let Ok(user) = gamma_client.get_me(access_cookie.value()).await {
			if user
				.groups
				.ok_or(Status::Forbidden)?
				.iter()
				.any(|g| g.name == GammaClient::owner_group())
			{
				Some(OwnerContext {
					years: get_years(&conn).await?,
				})
			} else {
				None
			}
		} else {
			None
		}
	} else {
		None
	};
	let context = create_base_context(
		SettingsContext {
			aoc_id,
			github,
			year,
			is_participating,
			owner,
		},
		cookies,
		gamma_client,
	)
	.await;
	Ok(Template::render("settings", context))
}

#[launch]
fn rocket() -> Rocket<Build> {
	rocket::build()
		.attach(Template::fairing())
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
		.mount("/static", FileServer::from("static/"))
		.mount("/", routes![
			index,
			login,
			about,
			settings,
			callback,
			callback_error,
			get_aoc_id_json,
			post_aoc_id_json,
			post_aoc_id,
			get_years_json,
			post_years_json,
			post_years,
			delete_years_json,
			delete_years,
			get_participate_json,
			post_participate_json,
			post_participate,
			delete_participate_json,
			delete_participate,
			get_leaderboard_year_json,
			get_leaderboard_year_splits_json,
			get_leaderboard_year_languages_json,
		])
}
