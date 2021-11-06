use std::{
	error::Error,
	fmt::{self, Display, Formatter},
};

use oauth2::{
	basic::{BasicClient, BasicErrorResponseType},
	reqwest::async_http_client,
	AuthUrl,
	AuthorizationCode,
	ClientId,
	ClientSecret,
	CsrfToken,
	ErrorResponse,
	RedirectUrl,
	RequestTokenError,
	StandardErrorResponse,
	TokenResponse,
	TokenUrl,
};
use reqwest::StatusCode;
use rocket::serde::Deserialize;

pub struct GammaClient {
	oauth_client: BasicClient,
	reqwest_client: reqwest::Client,
	api_base: String,
	api_key: String,
}

impl GammaClient {
	pub fn new(
		client_id: String,
		client_secret: String,
		redirect_url: String,
		callback_url: String,
		api_base: String,
		api_key: String,
	) -> Result<Self, String> {
		let auth_url = format!("{}/api/oauth/authorize", redirect_url);
		let token_url = format!("{}/oauth/token", api_base);
		Ok(Self {
			oauth_client: BasicClient::new(
				ClientId::new(client_id),
				Some(ClientSecret::new(client_secret)),
				AuthUrl::new(auth_url.clone()).map_err(|_| {
					format!(
						"Invalid authorization endpoint URL. (Attempt: \"{}\")",
						auth_url
					)
				})?,
				Some(TokenUrl::new(token_url.clone()).map_err(|_| {
					format!("Invalid token endpoint URL. (Attempt: \"{}\")", token_url)
				})?),
			)
			.set_redirect_uri(
				RedirectUrl::new(callback_url.clone()).map_err(|_| {
					format!("Invalid redirect URL. (Attempt: \"{}\")", callback_url)
				})?,
			),
			reqwest_client: reqwest::Client::new(),
			api_base,
			api_key,
		})
	}

	pub fn authorize_url(&self, back: String) -> String {
		let auth_url = self.oauth_client.authorize_url(|| CsrfToken::new(back));
		let (url, _) = auth_url.url();
		url.as_str().to_string()
	}

	pub async fn get_token(
		&self,
		code: String,
	) -> Result<
		String,
		GammaError<
			!,
			oauth2::reqwest::Error<reqwest::Error>,
			RequestTokenError<
				oauth2::reqwest::Error<reqwest::Error>,
				StandardErrorResponse<BasicErrorResponseType>,
			>,
			RequestTokenError<
				oauth2::reqwest::Error<reqwest::Error>,
				StandardErrorResponse<BasicErrorResponseType>,
			>,
		>,
	> {
		let token_response = self
			.oauth_client
			.exchange_code(AuthorizationCode::new(code))
			.request_async(async_http_client)
			.await
			.map_err(GammaError::from)?;
		Ok(token_response.access_token().secret().clone())
	}

	pub async fn get_me(
		&self,
		token: &str,
	) -> Result<ITUser, GammaError<reqwest::Error, reqwest::Error, reqwest::Error, reqwest::Error>>
	{
		self.reqwest_client
			.get(&format!("{}/users/me", self.api_base))
			.header("Authorization", format!("Bearer {}", token))
			.send()
			.await
			.map_err(GammaError::from)?
			.json::<ITUser>()
			.await
			.map_err(GammaError::from)
	}

	pub async fn get_user(
		&self,
		cid: &str,
	) -> Result<ITUser, GammaError<reqwest::Error, reqwest::Error, reqwest::Error, reqwest::Error>>
	{
		self.reqwest_client
			.get(&format!("{}/users/{}", self.api_base, cid))
			.header("Authorization", format!("pre-shared {}", self.api_key))
			.send()
			.await
			.map_err(GammaError::from)?
			.json::<ITUser>()
			.await
			.map_err(GammaError::from)
	}
}

#[derive(Debug)]
pub enum GammaError<
	TA: Error + 'static,
	TB: Error + 'static,
	TC: Error + 'static,
	TD: Error + 'static,
> {
	Unauthorized(TA),
	RequestError(TB),
	ClientError(TC),
	UnknownError(TD),
}

impl From<reqwest::Error>
	for GammaError<reqwest::Error, reqwest::Error, reqwest::Error, reqwest::Error>
{
	fn from(err: reqwest::Error) -> Self {
		if let Some(status) = err.status() {
			if status == StatusCode::UNAUTHORIZED {
				GammaError::Unauthorized(err)
			} else {
				GammaError::RequestError(err)
			}
		} else if err.is_decode() {
			GammaError::ClientError(err)
		} else {
			GammaError::UnknownError(err)
		}
	}
}

impl<TE: Error + 'static, TR: ErrorResponse> From<RequestTokenError<TE, TR>>
	for GammaError<!, TE, RequestTokenError<TE, TR>, RequestTokenError<TE, TR>>
{
	fn from(err: RequestTokenError<TE, TR>) -> Self {
		match err {
			RequestTokenError::ServerResponse(_) => GammaError::UnknownError(err),
			RequestTokenError::Request(err) => GammaError::RequestError(err),
			RequestTokenError::Parse(_, _) => GammaError::ClientError(err),
			RequestTokenError::Other(_) => GammaError::UnknownError(err),
		}
	}
}

impl<TA: Error + 'static, TB: Error + 'static, TC: Error + 'static, TD: Error + 'static> Error
	for GammaError<TA, TB, TC, TD>
{
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		Some(match self {
			GammaError::Unauthorized(err) => err,
			GammaError::RequestError(err) => err,
			GammaError::ClientError(err) => err,
			GammaError::UnknownError(err) => err,
		})
	}
}

impl<TA: Error, TB: Error, TC: Error, TD: Error> Display for GammaError<TA, TB, TC, TD> {
	fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
		match self {
			GammaError::Unauthorized(_) => write!(f, "Unauthorized"),
			GammaError::RequestError(_) => write!(f, "Request error"),
			GammaError::ClientError(_) => write!(f, "Client error"),
			GammaError::UnknownError(err) => write!(f, "Unknown error ({})", err),
		}
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ITUser {
	pub id: String,
	pub cid: String,
	pub nick: String,
	pub avatar_url: String,
	pub acceptance_year: u16,
	#[serde(default)]
	pub language: Option<String>,
	pub groups: Option<Vec<FKITGroup>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FKITGroup {
	pub id: String,
	pub becomes_active: u64,
	pub becomes_inactive: u64,
	pub name: String,
	pub pretty_name: String,
	pub avatar_url: Option<String>,
	pub super_group: FKITSuperGroup,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FKITSuperGroup {
	pub id: String,
	pub name: String,
	pub pretty_name: String,
	pub r#type: GroupType,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum GroupType {
	Committee,
	Society,
	Alumni,
}
