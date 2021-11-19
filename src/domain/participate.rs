use diesel::{expression_methods::ExpressionMethods, query_dsl::QueryDsl, RunQueryDsl};
use rocket::{
	http::{CookieJar, Status},
	serde::{Deserialize, Serialize},
};

use crate::{
	db::{participants, DbConn, Participant},
	gamma::GammaClient,
};

pub async fn get_participations(
	conn: &DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<Vec<ParticipateResponse>, Status> {
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
	Ok(participant
		.drain(..)
		.map(|p| ParticipateResponse {
			year: p.year,
			github: p.github,
		})
		.collect())
}

pub async fn set_participation(
	data: ParticipateRequest,
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
	conn.run(move |c| {
		diesel::insert_into(participants::table)
			.values(Participant {
				cid: user.cid,
				year: data.year,
				github: data.github.clone(),
			})
			.on_conflict((participants::columns::cid, participants::columns::year))
			.do_update()
			.set(participants::columns::github.eq(data.github))
			.execute(c)
	})
	.await
	.map_err(|_| Status::InternalServerError)?;
	Ok(())
}

pub async fn delete_participation(
	data: ParticipateDeleteRequest,
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
		Ok(())
	} else {
		Err(Status::NotFound)
	}
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipateRequest {
	pub year: i32,
	#[serde(default)]
	pub github: Option<String>,
}

pub type ParticipateResponse = ParticipateRequest;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipateDeleteRequest {
	pub year: i32,
}
