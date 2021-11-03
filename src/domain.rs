use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AocIdRequest {
	pub aoc_id: String,
}

pub type AocIdResponse = AocIdRequest;
