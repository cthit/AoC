#![feature(never_type, proc_macro_hygiene, decl_macro)]

mod db;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;

use db::DbConn;
use rocket::{fairing::AdHoc, Build, Rocket};

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
		.mount("/", routes![index])
}
