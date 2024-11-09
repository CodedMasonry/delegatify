pub mod commands;
pub mod database;
pub mod spotify;

use rspotify::AuthCodePkceSpotify;
use tokio::sync::RwLock;

// User data, which is stored and accessible in all command invocations
pub struct Data {
    pub spotify: RwLock<Option<AuthCodePkceSpotify>>,
    pub pool: sqlx::PgPool,
    pub freeze: RwLock<bool>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub fn format_delta(time: chrono::TimeDelta) -> String {
    let total_seconds = time.num_seconds();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

pub async fn is_frozen(ctx: Context<'_>) -> bool {
    *ctx.data().freeze.read().await
}
