#![feature(never_type, proc_macro_hygiene, decl_macro)]

mod db;
mod gamma;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;

use std::env;

use db::DbConn;
use gamma::GammaClient;
use lazy_static::lazy_static;
use rocket::{
	fairing::AdHoc,
	http::{Cookie, CookieJar},
	response::Redirect,
	Build,
	Rocket,
};

static GAMMA_COOKIE: &str = "gamma";
lazy_static! {
	static ref GAMMA_CLIENT_ID: String =
		env::var("GAMMA_CLIENT_ID").expect("Missing the GAMMA_CLIENT_ID environment variable.");
	static ref GAMMA_CLIENT_SECRET: String = env::var("GAMMA_CLIENT_SECRET")
		.expect("Missing the GAMMA_CLIENT_SECRET environment variable.");
	static ref GAMMA_REDIRECT_URL: String = env::var("GAMMA_REDIRECT_URL")
		.expect("Missing the GAMMA_REDIRECT_URL environment variable.");
	static ref CALLBACK_URL: String =
		env::var("CALLBACK_URL").expect("Missing the CALLBACK_URL environment variable.");
	static ref GAMMA_URL: String = format!(
		"{}/api",
		env::var("GAMMA_URL").expect("Missing the GAMMA_URL environment variable.")
	);
	static ref OAUTH_CLIENT: GammaClient = GammaClient::new(
		GAMMA_CLIENT_ID.to_string(),
		GAMMA_CLIENT_SECRET.to_string(),
		GAMMA_REDIRECT_URL.to_string(),
		CALLBACK_URL.to_string(),
		GAMMA_URL.to_string(),
	)
	.unwrap_or_else(|e| panic!("Failed to create the OAuth client. {}", e));
}

#[get("/login?<back>")]
async fn login(back: Option<String>, cookies: &CookieJar<'_>) -> Redirect {
	if let Some(access_cookie) = cookies.get(GAMMA_COOKIE) {
		if OAUTH_CLIENT
			.get_user(access_cookie.value().to_string())
			.await
			.is_ok()
		{
			return Redirect::to(back.unwrap_or_else(|| "/".to_string()));
		}
	}
	Redirect::to(OAUTH_CLIENT.authorize_url(back.unwrap_or_else(|| "/".to_string())))
}

#[get("/callback?<code>&<state>")]
async fn callback(code: String, state: Option<String>, cookies: &CookieJar<'_>) -> Redirect {
	match OAUTH_CLIENT.get_token(code).await {
		Ok(access_token) => match OAUTH_CLIENT.get_user(access_token.clone()).await {
			Ok(_) => {
				cookies.add(Cookie::build(GAMMA_COOKIE, access_token).path("/").finish());
				Redirect::to(state.unwrap_or_else(|| "/".to_string()))
			}
			Err(err) => Redirect::to(uri!(unauthorized(Some(err.to_string())))),
		},
		Err(err) => Redirect::to(uri!(unauthorized(Some(err.to_string())))),
	}
}

#[get("/callback?<error>&<error_description>", rank = 2)]
async fn callback_error(error: String, error_description: Option<String>) -> String {
	match error_description {
		Some(desc) => format!("{}\n{}", error, desc),
		None => error,
	}
}

#[get("/unauthorized?<reason>")]
fn unauthorized(reason: Option<String>) -> String {
	reason.unwrap_or_else(|| "Unauthorized".to_string())
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
			unauthorized,
		])
}
