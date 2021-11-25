use diesel::{expression_methods::ExpressionMethods, query_dsl::QueryDsl, RunQueryDsl};
use rocket::{
	form::FromForm,
	http::{CookieJar, Status},
	serde::{Deserialize, Serialize},
};

use crate::{
	db::{years, DbConn, Year},
	gamma::GammaClient,
};

pub async fn get_years(conn: &DbConn) -> Result<Vec<YearResponse>, Status> {
	let mut years_db: Vec<Year> = conn
		.run(move |c| years::table.load(c))
		.await
		.map_err(|_| Status::InternalServerError)?;
	Ok(years_db
		.drain(..)
		.map(|y| YearResponse {
			year: y.year,
			leaderboard: y.leaderboard_id().to_owned(),
		})
		.collect())
}

pub async fn get_year(
	year: i32,
	conn: &DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<YearResponse, Status> {
	let access_cookie = cookies
		.get(GammaClient::cookie())
		.ok_or(Status::Unauthorized)?;
	let _user = gamma_client
		.get_me(access_cookie.value())
		.await
		.map_err(|_| Status::Unauthorized)?;

	let year: Year = conn
		.run(move |c| years::table.filter(years::columns::year.eq(year)).first(c))
		.await
		.map_err(|_| Status::InternalServerError)?;
	Ok(YearResponse {
		year: year.year,
		leaderboard: year.leaderboard,
	})
}

pub async fn set_year(
	data: YearRequest,
	conn: &DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<(), Status> {
	let leaderboard_split: Vec<_> = data.leaderboard.split('-').collect();
	if leaderboard_split.len() != 2
		|| !leaderboard_split[0].chars().all(char::is_numeric)
		|| !leaderboard_split[1].chars().all(char::is_alphanumeric)
	{
		return Err(Status::BadRequest);
	}

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
			.set(years::columns::leaderboard.eq(data.leaderboard))
			.execute(c)
	})
	.await
	.map_err(|_| Status::InternalServerError)?;
	Ok(())
}

pub async fn delete_year(
	data: YearDeleteRequest,
	conn: &DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<(), Status> {
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
		Ok(())
	} else {
		Err(Status::NotFound)
	}
}

#[derive(Deserialize, FromForm, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YearRequest {
	pub year: i32,
	pub leaderboard: String,
}

pub type YearResponse = YearRequest;

#[derive(Deserialize, FromForm)]
#[serde(rename_all = "camelCase")]
pub struct YearDeleteRequest {
	pub year: i32,
}
