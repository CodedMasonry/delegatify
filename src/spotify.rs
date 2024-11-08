use chrono::TimeDelta;
use rspotify::{
    model::{FullEpisode, FullTrack, PlayableItem},
    scopes, AuthCodePkceSpotify, OAuth,
};
use serde::{Deserialize, Serialize};

use crate::Context;

pub struct StandardItem {
    pub name: String,
    pub duration: TimeDelta,
    pub artists: Vec<String>,
    pub image: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaybackStateResponse {
    pub progress_ms: Option<i32>,
    pub is_playing: bool,
    pub item: serde_json::Value,
    pub currently_playing_type: String,
}

impl StandardItem {
    pub async fn parse(item: &PlayableItem) -> StandardItem {
        match item {
            PlayableItem::Track(track) => handle_track_current(track).await,
            PlayableItem::Episode(episode) => handle_episode_current(episode).await,
        }
    }
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

pub async fn handle_track_current(track: &FullTrack) -> StandardItem {
    let image = match track.album.images.get(0) {
        Some(v) => v.url.clone(),
        None => String::new(),
    };
    let artists = track
        .artists
        .iter()
        .map(|artist| artist.name.clone())
        .collect::<Vec<String>>();
    let url = track.external_urls.get("spotify").unwrap().clone();

    StandardItem {
        name: track.name.clone(),
        duration: track.duration.clone(),
        artists,
        image,
        url,
    }
}

pub async fn handle_episode_current(track: &FullEpisode) -> StandardItem {
    let image = match track.images.get(0) {
        Some(v) => v.url.clone(),
        None => String::new(),
    };
    let url = track.external_urls.get("spotify").unwrap().clone();

    StandardItem {
        name: track.name.clone(),
        duration: track.duration.clone(),
        artists: vec![track.show.name.clone()],
        image,
        url,
    }
}
