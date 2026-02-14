use crate::models::DeviceFlowInfo;
use serde::Deserialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Device code request failed: {0}")]
    DeviceCodeRequest(String),
}

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const CLIENT_ID: &str = "Iv23licutsPQwDRYefce";

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

pub async fn init_auth_flow(app_handle: &AppHandle) -> Result<DeviceFlowInfo> {
    log::info!("Initializing GitHub Device Flow");

    let client = reqwest::Client::new();

    let response = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", "repo")])
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::DeviceCodeRequest(body));
    }

    let device_response: DeviceCodeResponse = response.json().await?;

    let info = DeviceFlowInfo {
        user_code: device_response.user_code.clone(),
        verification_uri: device_response.verification_uri.clone(),
    };

    let _ = app_handle
        .opener()
        .open_url(&device_response.verification_uri, None::<String>);

    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        poll_for_token(
            &app_handle,
            &client,
            &device_response.device_code,
            device_response.interval,
            device_response.expires_in,
        )
        .await;
    });

    Ok(info)
}

async fn poll_for_token(
    app_handle: &AppHandle,
    client: &reqwest::Client,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) {
    let started = std::time::Instant::now();
    let mut interval_secs = interval;

    loop {
        if started.elapsed().as_secs() > expires_in {
            log::error!("Device code expired");
            let _ = app_handle.emit("auth-error", "Device code expired");
            return;
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

        let response = client
            .post(TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await;

        let resp = match response {
            Ok(resp) => resp,
            Err(err) => {
                log::error!("HTTP request failed during polling: {err}");
                continue;
            }
        };

        let token_resp = match resp.json::<TokenResponse>().await {
            Ok(parsed) => parsed,
            Err(err) => {
                log::error!("Failed to parse token response: {err}");
                continue;
            }
        };

        if let Some(access_token) = token_resp.access_token {
            log::info!("Got GitHub access token via device flow");
            if let Err(err) = app_handle.emit("auth-token", &access_token) {
                log::error!("Failed to emit auth token: {err}");
            }
            return;
        }

        match token_resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval_secs += 5;
                continue;
            }
            Some("expired_token") => {
                log::error!("Device code expired");
                let _ = app_handle.emit("auth-error", "Device code expired");
                return;
            }
            Some("access_denied") => {
                log::error!("User denied access");
                let _ = app_handle.emit("auth-error", "Access denied");
                return;
            }
            Some(err) => {
                log::error!("Token polling error: {err}");
                let _ = app_handle.emit("auth-error", err);
                return;
            }
            None => {
                log::error!("Unexpected response without access_token or error");
                continue;
            }
        }
    }
}
