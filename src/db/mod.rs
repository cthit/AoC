use diesel::{
	connection::SimpleConnection,
	table,
	Associations,
	Identifiable,
	Insertable,
	PgConnection,
	Queryable,
};
use rocket_sync_db_pools::database;

#[database("aoc")]
pub struct DbConn(PgConnection);

table! {
	users (cid) {
		cid -> Text,
		aoc_id -> Text,
	}
}

#[derive(Identifiable, Insertable, Queryable)]
#[primary_key(cid)]
#[table_name = "users"]
pub struct User {
	pub cid: String,
	pub aoc_id: String,
}

table! {
	years (year) {
		year -> Integer,
		leaderboard -> Text,
	}
}

#[derive(Identifiable, Insertable, Queryable)]
#[primary_key(year)]
#[table_name = "years"]
pub struct Year {
	pub year: i32,
	pub leaderboard: String,
}

table! {
	participants (cid, year) {
		cid -> Text,
		year -> Integer,
		github -> Nullable<Text>,
	}
}

#[derive(Associations, Identifiable, Insertable, Queryable)]
#[primary_key(cid, year)]
#[belongs_to(User, foreign_key = "cid")]
#[belongs_to(Year, foreign_key = "year")]
#[table_name = "participants"]
pub struct Participant {
	pub cid: String,
	pub year: i32,
	pub github: Option<String>,
}

pub async fn initialize(rocket: &rocket::Rocket<rocket::Orbit>) -> Result<(), String> {
	let conn = DbConn::get_one(rocket)
		.await
		.ok_or_else(|| "Database connection not found.".to_string())?;
	conn.run(|c| c.batch_execute(include_str!("./schema.sql")))
		.await
		.map_err(|e| format!("Error creating tables: {}", e))?;
	Ok(())
}
