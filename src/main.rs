use anyhow::Context as _;
use delegatify::spotify;
use parking_lot::Mutex;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use poise::Modal;
use rspotify::prelude::OAuthClient;
use rspotify::AuthCodeSpotify;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use tracing::info;

// User data, which is stored and accessible in all command invocations
struct Data {
    client_id: String,
    client_secret: String,
    client: reqwest::Client,
    spotify: Mutex<Option<AuthCodeSpotify>>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
type ApplicationContext<'a> = poise::ApplicationContext<'a, Data, Error>;

/// Modal for authentication
#[derive(Debug, Modal)]
#[name = "Spotify Authentication"]
struct SpotifyAuthenticationModal {
    #[name = "Paste the code that you recieved here"]
    #[min_length = 5]
    #[max_length = 500]
    code: String,
}

#[poise::command(slash_command)]
async fn authenticate(ctx: ApplicationContext<'_>) -> Result<(), Error> {
    let spotify = spotify::init(&ctx.data().client_id, &ctx.data().client_secret).await?;
    let url = spotify.get_authorize_url(false).unwrap();

    ctx.reply(format!("# [Click Here]({url}) to authenticate"))
        .await?;

    let data: SpotifyAuthenticationModal = SpotifyAuthenticationModal::execute(ctx).await?.unwrap();

    spotify.request_token(&data.code).await?;
    *ctx.data.spotify.lock() = Some(spotify);
    Ok(())
}

#[poise::command(slash_command)]
async fn current(ctx: Context<'_>) -> Result<(), Error> {
    let info = match ctx.data().spotify.lock() {
        Some(v) => todo!(),
        None => {
            todo!()
        }
    };

    ctx.say(msg).await?;
    Ok(())
}

async fn error_unauthorized(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let client_id = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let client_secret = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![authenticate(), current()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    client_id,
                    client_secret,
                    client: reqwest::Client::new(),
                    spotify: Mutex::new(None),
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
