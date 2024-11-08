use crate::{spotify, Context, Error};
use poise::serenity_prelude::{self as serenity, Colour, CreateEmbed, Timestamp};
use poise::Modal;
use rspotify::prelude::OAuthClient;
use tracing::{debug, info};

/// Modal for authentication
#[derive(Debug, Modal)]
#[name = "Spotify Authentication"]
struct SpotifyAuthenticationModal {
    #[name = "Paste the code that you recieved here"]
    #[min_length = 64]
    #[max_length = 512]
    code: String,
}

/// Authenticates the application with specified token
#[poise::command(slash_command, owners_only)]
pub async fn authenticate(ctx: Context<'_>) -> Result<(), Error> {
    let mut spotify = spotify::init().await?;
    let url = spotify.get_authorize_url(None).unwrap();

    let reply = {
        let components = vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new_link(url)
                .label("Open URL")
                .style(poise::serenity_prelude::ButtonStyle::Primary),
            serenity::CreateButton::new("open_modal")
                .label("Authenticate")
                .style(poise::serenity_prelude::ButtonStyle::Success),
        ])];

        poise::CreateReply::default()
            .ephemeral(true)
            .embed(CreateEmbed::new()
            .color(Colour::BLUE)
            .timestamp(Timestamp::now())
            .title("Authenticating Delegatify")
            .description(
                    "In order for the application to work, a spotify account must be connected",
            )
            .field("Open URL Button", "This button opens a link to recieve an authentication code. When you recieve the code, click on the Authenticate button.", false)
            .field("Authenticate Button", "This is the button you click when you have the code. It will ask you to input the code, and then you are good to go.", false))
            .components(components)
    };

    ctx.send(reply).await?;

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(std::time::Duration::from_secs(120))
        .filter(move |mci| mci.data.custom_id == "open_modal")
        .await
    {
        let data = poise::execute_modal_on_component_interaction::<SpotifyAuthenticationModal>(
            ctx, mci, None, None,
        )
        .await?;

        if let Some(v) = data {
            info!("Recieved Code");
            let borrow = &spotify;
            borrow
                .request_token(&v.code)
                .await
                .map_err(|err| format!("Failed to Authenticate:\n{err}"))?;
            debug!("Requested Token");

            ctx.reply(format!("Successfully Authenticated!")).await?;

            *ctx.data().spotify.write().await = Some(spotify.clone());
        } else {
            ctx.reply("No Input provided").await?;
        }
    }
    Ok(())
}

/// Check the current playback
#[poise::command(slash_command, user_cooldown = 10)]
pub async fn current(ctx: Context<'_>) -> Result<(), Error> {
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(());
        }
    };

    let data = match client.current_playback(None, None::<Vec<_>>).await? {
        Some(v) => v,
        None => {
            ctx.say("Nothing Playing").await?;
            return Ok(());
        }
    };

    let item = match data.item {
        Some(v) => v,
        None => {
            ctx.say("Nothing Playing").await?;
            return Ok(());
        }
    };

    let reply = poise::CreateReply::default().embed(
        CreateEmbed::new()
            .color(Colour::BLUE)
            .timestamp(Timestamp::now())
            .title(format!("{:?}", item)),
    );

    ctx.send(reply).await?;
    Ok(())
}

/// Error 401 response for discord
pub async fn error_unauthorized(ctx: Context<'_>) -> Result<(), Error> {
    ctx.reply("The application isn't authenticated.\nrun '/authenticate' to connect.")
        .await?;
    Ok(())
}
