use diesel::{expression_methods::ExpressionMethods, query_dsl::QueryDsl, RunQueryDsl};
use rocket::{
	http::{CookieJar, Status},
	serde::{Deserialize, Serialize},
};

use crate::{
	db::{users, DbConn, User},
	gamma::GammaClient,
};

pub async fn get_aoc_id(
	conn: &DbConn,
	cookies: &CookieJar<'_>,
	gamma_client: &GammaClient,
) -> Result<AocIdResponse, Status> {
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
	Ok(AocIdResponse {
		aoc_id: user_db.aoc_id,
	})
}

pub async fn set_aoc_id(
	aoc_id: String,
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
		diesel::insert_into(users::table)
			.values(User {
				cid: user.cid,
				aoc_id: aoc_id.clone(),
			})
			.on_conflict(users::columns::cid)
			.do_update()
			.set(users::columns::aoc_id.eq(aoc_id))
			.execute(c)
	})
	.await
	.map_err(|_| Status::InternalServerError)?;
	Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AocIdRequest {
	pub aoc_id: String,
}

pub type AocIdResponse = AocIdRequest;
