use rspotify::{scopes, AuthCodePkceSpotify, OAuth};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaybackStateResponse {
    pub progress_ms: Option<i32>,
    pub is_playing: bool,
    pub item: serde_json::Value,
    pub currently_playing_type: String,
}

pub async fn init() -> Result<rspotify::AuthCodePkceSpotify, anyhow::Error> {
    let creds = rspotify::Credentials::from_env().expect("Credentials Not Provided");

    // Using every possible scope
    let scopes = scopes!(
        "user-read-playback-state",
        "user-read-currently-playing",
        "user-modify-playback-state",
        "user-read-recently-played"
    );
    let oauth = OAuth::from_env(scopes).unwrap();
    let config = rspotify::Config {
        ..Default::default()
    };

    Ok(AuthCodePkceSpotify::with_config(
        creds.clone(),
        oauth.clone(),
        config.clone(),
    ))
}