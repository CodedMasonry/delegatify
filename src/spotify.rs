use chrono::TimeDelta;
use rspotify::{
    model::{FullEpisode, FullTrack, PlayableItem, TrackId},
    prelude::{BaseClient, OAuthClient},
    scopes, AuthCodePkceSpotify, OAuth,
};
use serde::{Deserialize, Serialize};

use crate::{Context, Error};

pub struct StandardItem {
    pub name: String,
    pub duration: TimeDelta,
    pub artists: Vec<String>,
    pub image: String,
    pub url: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaybackStateResponse {
    pub progress_ms: Option<i32>,
    pub is_playing: bool,
    pub item: serde_json::Value,
    pub currently_playing_type: String,
}

impl StandardItem {
    pub fn parse(item: &PlayableItem) -> StandardItem {
        match item {
            PlayableItem::Track(track) => handle_track_current(track),
            PlayableItem::Episode(episode) => handle_episode_current(episode),
        }
    }
}

pub async fn init() -> Result<rspotify::AuthCodePkceSpotify, Error> {
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
        token_refreshing: true,
        ..Default::default()
    };

    Ok(AuthCodePkceSpotify::with_config(
        creds.clone(),
        oauth.clone(),
        config.clone(),
    ))
}

pub async fn fetch_queue(ctx: Context<'_>) -> Result<Vec<StandardItem>, Error> {
    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            return Err("Unauthorized".into());
        }
    };

    let data = client.current_user_queue().await?.queue;
    // Free client lock
    drop(lock);

    let mut queue = Vec::new();
    for item in data {
        let value = StandardItem::parse(&item);
        queue.push(value);
    }

    Ok(queue)
}

pub async fn fetch_track(ctx: Context<'_>, track: TrackId<'_>) -> Result<StandardItem, Error> {
    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            return Err("Unauthorized".into());
        }
    };

    let data = client.track(track, None).await?;
    // Free client lock
    drop(lock);

    let data = StandardItem::parse(&PlayableItem::Track(data));
    Ok(data)
}

pub fn handle_track_current(track: &FullTrack) -> StandardItem {
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
        id: track.id.clone().unwrap().to_string(),
    }
}

pub fn handle_episode_current(track: &FullEpisode) -> StandardItem {
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
        id: track.id.to_string(),
    }
}
