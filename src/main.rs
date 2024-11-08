use std::collections::HashSet;
#[deny(clippy::all)]
use std::env;

use anyhow::Context as _;
use delegatify::{commands::{authenticate, current}, Data};
use poise::serenity_prelude::{ClientBuilder, GatewayIntents, UserId};
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use tokio::sync::RwLock;

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    // Discord Secrets
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let dev_user = secret_store
        .get("DISCORD_DEV_ID")
        .context("'DISCORD_DEV_ID' was not found")?
        .parse::<u64>()
        .context("Failed to parse Developer ID, make sure it is a valid number")?;

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

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![authenticate(), current()],
            owners: HashSet::from([UserId::new(dev_user)]),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    spotify: RwLock::new(None),
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
