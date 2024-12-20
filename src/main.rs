#[deny(clippy::all)]
use std::env;

use anyhow::Context as _;
use delegatify::{
    commands::{add_user, authenticate, current, freeze, next, play, previous, queue, remove_user},
    database, Data,
};
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use tokio::sync::RwLock;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres] pool: sqlx::PgPool,
) -> ShuttleSerenity {
    // Discord Secrets
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    // Spotify Secrets
    let client_id = secret_store
        .get("SPOTIFY_CLIENT_ID")
        .context("'CLIENT_ID' was not found")?;
    let client_secret = secret_store
        .get("SPOTIFY_CLIENT_SECRET")
        .context("'CLIENT_SECRET' was not found")?;
    let callback_url = secret_store
        .get("SPOTIFY_REDIRECT_URI")
        .context("'SPOTIFY_REDIRECT_URI' was not found")?;

    // set ENV variables for rspotify
    env::set_var("RSPOTIFY_CLIENT_ID", client_id.clone());
    env::set_var("RSPOTIFY_CLIENT_SECRET", client_secret.clone());
    env::set_var("RSPOTIFY_REDIRECT_URI", callback_url.clone());

    // Handle migrations
    database::migrate(&pool)
        .await
        .context("Failed to migrate Database")?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                // Playback
                current(),
                queue(),
                play(),
                previous(),
                next(),
                // Utilities
                freeze(),
                add_user(),
                remove_user(),
                authenticate(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    spotify: RwLock::new(None),
                    pool,
                    freeze: RwLock::new(false),
                })
            })
        })
        .build();

    let client = ClientBuilder::new(discord_token, GatewayIntents::non_privileged())
        .framework(framework)
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
