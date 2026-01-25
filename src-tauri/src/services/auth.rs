use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse, TokenUrl,
};
use obfstr::obfstring;
use tauri::{AppHandle, Emitter, Url};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to open URL: {0}")]
    OpenUrl(String),
}

pub struct AuthService;

const AUTH_URI: &str = "https://github.com/login/oauth/authorize";
const TOKEN_URI: &str = "https://github.com/login/oauth/access_token";
const CLIENT_ID: &str = "Iv23licutsPQwDRYefce";
const REDIRECT_URL: &str = "pr-manager://oauth";

impl AuthService {
    pub fn init_auth_flow(app_handle: &AppHandle) -> Result<()> {
        log::info!("Initializing GitHub OAuth flow");

        let client = BasicClient::new(ClientId::new(CLIENT_ID.into()))
            .set_client_secret(ClientSecret::new(
                // NOTE: This only makes it harder to find clinet secret.
                // However we cannot remove this because github api requires client secret
                // even when using oauth2.0 pkce workflow enventhoug spec allow us to omit it.
                obfstring!(std::env!("GITHUB_APP_CLIENT_SECRET")),
            ))
            .set_token_uri(TokenUrl::new(TOKEN_URI.into()).expect("Should parse token uri"))
            .set_auth_uri(AuthUrl::new(AUTH_URI.into()).expect("Should parse"))
            .set_redirect_uri(
                RedirectUrl::new(REDIRECT_URL.into()).expect("Why would this ever fail"),
            );

        let (code_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(code_challenge)
            .url();

        log::debug!("Generated auth URL: {}", auth_url);

        app_handle
            .opener()
            .open_url(auth_url, None::<String>)
            .map_err(|err| Error::OpenUrl(err.to_string()))?;

        let app_handle = app_handle.clone();
        log::info!("Stating to listen for deep_link");
        app_handle.clone().deep_link().on_open_url(move |event| {
            let code_and_state = event
                .urls()
                .iter()
                .filter_map(Self::extract_code_and_state)
                .next();
            let Some((code, state)) = code_and_state else {
                return;
            };

            if state != csrf_token {
                log::error!("Got an invalid state");
                return;
            }

            {
                let pkce_verifier = PkceCodeVerifier::new(pkce_verifier.secret().into());
                let client = client.clone();
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let http_client = oauth2::reqwest::Client::new();
                    let res = client
                        .clone()
                        .exchange_code(code)
                        .set_pkce_verifier(pkce_verifier)
                        .request_async(&http_client)
                        .await;
                    match res {
                        Ok(token_response) => {
                            log::info!("Got github access token");
                            let access_token = token_response.access_token();

                            // Emit token to frontend for storage
                            if let Err(err) = app_handle.emit("auth-token", access_token.secret()) {
                                log::error!("Failed to emit auth token: {err}");
                            }
                        }
                        Err(err) => {
                            log::error!("Failed to get token: {:?}", err);
                        }
                    }
                });
            }
        });

        Ok(())
    }

    fn extract_code_and_state(url: &Url) -> Option<(AuthorizationCode, CsrfToken)> {
        if url.scheme() != "pr-manager" || url.host_str() != Some("oauth") {
            log::warn!("Got unkdown url: {url}");
            return None;
        }
        let mut params = url.query_pairs();
        let code = params.clone().find(|(key, _)| key == "code");
        let Some((_, code)) = code else {
            log::error!("Got callback url without code url: {url}");
            return None;
        };
        let state = params.find(|(key, _)| key == "state");
        let Some((_, state)) = state else {
            log::error!("Got callback url without state. url: {url}");
            return None;
        };
        Some((
            AuthorizationCode::new(code.into()),
            CsrfToken::new(state.into()),
        ))
    }
}
