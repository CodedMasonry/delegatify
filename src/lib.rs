pub mod spotify;
pub mod commands;

use rspotify::AuthCodePkceSpotify;
use tokio::sync::RwLock;

// User data, which is stored and accessible in all command invocations
pub struct Data {
    pub spotify: RwLock<Option<AuthCodePkceSpotify>>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
