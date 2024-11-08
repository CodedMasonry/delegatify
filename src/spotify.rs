use rspotify::{scopes, AuthCodeSpotify, OAuth};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaybackStateResponse {
    pub progress_ms: Option<i32>,
    pub is_playing: bool,
    pub item: serde_json::Value,
    pub currently_playing_type: String,
}

pub async fn init(
    client_id: &str,
    client_secret: &str,
) -> Result<rspotify::AuthCodeSpotify, anyhow::Error> {
    let creds = rspotify::Credentials::new(client_id, client_secret);

    // Using every possible scope
    let scopes = scopes!(
        "user-read-recently-played",
        "user-read-currently-playing",
        "user-read-playback-state",
        "user-read-playback-position",
        "user-modify-playback-state"
    );
    let oauth = OAuth::from_env(scopes).unwrap();
    let config = rspotify::Config::default();

    Ok(AuthCodeSpotify::with_config(
        creds.clone(),
        oauth.clone(),
        config.clone(),
    ))
}
