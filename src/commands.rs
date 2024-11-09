use crate::spotify::{fetch_queue, fetch_track, StandardItem};
use crate::{format_delta, spotify, Context, Error};
use poise::serenity_prelude::{
    self as serenity, Colour, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, Timestamp,
};
use poise::{CreateReply, Modal};
use rspotify::model::{
    CurrentPlaybackContext, IdError, PlayableId, PlayableItem, RepeatState, SearchResult,
    SearchType, TrackId,
};
use rspotify::prelude::{BaseClient, OAuthClient};
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
    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(());
        }
    };

    // Get the playback state
    let playback = match client.current_playback(None, None::<Vec<_>>).await? {
        Some(v) => v,
        None => {
            ctx.say("Nothing Playing").await?;
            return Ok(());
        }
    };
    // Force drop to allow for other requests
    drop(lock);

    let embed = CreateEmbed::new();

    // Check if something is actually playing
    let embed = match &playback.item {
        Some(item) => current_playback(&playback, item, embed).await,
        None => current_no_playback(embed).await,
    };

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

async fn current_playback(
    playback: &CurrentPlaybackContext,
    item: &PlayableItem,
    embed: CreateEmbed,
) -> CreateEmbed {
    let item = StandardItem::parse(item).await;

    let progress = playback.progress.unwrap();
    let duration = format!(
        "{} / {}",
        format_delta(progress),
        format_delta(item.duration)
    );
    let shuffle = if playback.shuffle_state { "On" } else { "Off" };
    let repeat = match playback.repeat_state {
        RepeatState::Off => "Off",
        RepeatState::Track => "Track",
        RepeatState::Context => "Context",
    };
    // Create Embed
    embed
        .color(Colour::DARK_GREEN)
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Delegatify"))
        .author(CreateEmbedAuthor::new("Currenting Playing..."))
        .title(format!("{} - {}", item.name, item.artists.join(", ")))
        .thumbnail(item.image)
        .field("Time", duration, false)
        .field("Shuffle", shuffle, true)
        .field("Repeat", repeat, true)
}

async fn current_no_playback(embed: CreateEmbed) -> CreateEmbed {
    // Create Embed
    embed
        .color(Colour::DARK_RED)
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Delegatify"))
        .title("Nothing Playing")
        .description("Nothing is currently being played ")
}

/// Check the queue
#[poise::command(slash_command, user_cooldown = 10)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let data = fetch_queue(ctx).await?;
    let mut queue = Vec::new();
    for (index, value) in data.into_iter().enumerate() {
        // Limit queue to 5 results
        if index == 5 {
            break;
        }

        queue.push(format!(
            "**[{}]({})**\n{}",
            value.name,
            value.url,
            value.artists.join(", "),
        ));
    }

    let embed = CreateEmbed::new()
        .colour(Colour::DARK_GREEN)
        .title("Current Queue")
        .description("The next five songs that are in the queue.")
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Delegatify"))
        .description(format!("{}\n**...**", queue.join("\n\n")));

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Add a song to the queue
#[poise::command(slash_command, user_cooldown = 30)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Either the URL or search query"]
    #[max_length = 512]
    input: String,
) -> Result<(), Error> {
    if is_frozen(ctx).await {
        ctx.say("Playback changes are frozen").await?;
        return Ok(());
    }

    let id = if input.starts_with("https") && input.contains("track") && input.contains("spotify") {
        play_url(&input).await?
    } else {
        play_search(ctx, input).await?
    };

    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(());
        }
    };

    client
        .add_item_to_queue(PlayableId::Track(id.clone()), None)
        .await?;
    drop(lock);

    let track = fetch_track(ctx, id).await?;
    let embed = CreateEmbed::new()
        .colour(Colour::DARK_GREEN)
        .author(CreateEmbedAuthor::new("Added Song To Queue"))
        .title(format!("{} - {}", track.name, track.artists.join(", ")))
        .thumbnail(track.image)
        .field(
            "Length",
            format!("{}s", format_delta(track.duration)),
            false,
        )
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(format!("Requested by <@{}>", ctx.author().id))
                .icon_url(ctx.author().avatar_url().unwrap_or_default()),
        );

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

async fn play_url<'a>(url: &'a str) -> Result<TrackId<'a>, IdError> {
    let id = url.split('/').last().unwrap().split('?').next().unwrap();
    TrackId::from_id(id)
}

async fn play_search<'a>(ctx: Context<'_>, input: String) -> Result<TrackId<'a>, Error> {
    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Err("Unauthorized".into());
        }
    };

    let search_result = client
        .search(&input, SearchType::Track, None, None, Some(1), None)
        .await?;
    // Free client
    drop(lock);

    let data = if let SearchResult::Tracks(page) = search_result {
        match page.items.first() {
            Some(item) => item.clone(),
            None => return Err("Couldn't find a result".into()),
        }
    } else {
        panic!("Not Possible")
    };

    let id = data.id.clone().unwrap();
    let track = StandardItem::parse(&PlayableItem::Track(data)).await;

    let reply = {
        let components = vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new("accept")
                .label("Add")
                .style(poise::serenity_prelude::ButtonStyle::Success),
            serenity::CreateButton::new("cancel")
                .label("Cancel")
                .style(poise::serenity_prelude::ButtonStyle::Danger),
        ])];

        poise::CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .color(Colour::BLUE)
                    .timestamp(Timestamp::now())
                    .author(CreateEmbedAuthor::new("Want To Add This Song?"))
                    .title(format!("{} - {}", track.name, track.artists.join(", ")))
                    .thumbnail(track.image)
                    .field(
                        "Length",
                        format!("{}s", format_delta(track.duration)),
                        false,
                    )
                    .footer(CreateEmbedFooter::new("Delegatify")),
            )
            .components(components)
    };

    ctx.send(reply).await?;

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(std::time::Duration::from_secs(120))
        .filter(move |mci| mci.data.custom_id == "accept" || mci.data.custom_id == "cancel")
        .await
    {
        if mci.data.custom_id == "accept" {
            return Ok(id);
        } else {
            return Err("Cancelled Interaction".into());
        }
    }

    Err("Failed to handle interactions".into())
}

/// Switch the state of freeze
#[poise::command(slash_command, owners_only)]
pub async fn freeze(ctx: Context<'_>) -> Result<(), Error> {
    let mut v = ctx.data().freeze.write().await;

    if *v {
        *v = false;
        ctx.say("Disabled Freeze").await?;
    } else {
        *v = true;
        ctx.say("Enabled Freeze").await?;
    }

    Ok(())
}

pub async fn is_frozen(ctx: Context<'_>) -> bool {
    *ctx.data().freeze.read().await
}

/// Error 401 response for discord
pub async fn error_unauthorized(ctx: Context<'_>) -> Result<(), Error> {
    ctx.reply("The application isn't authenticated.\nrun '/authenticate' to connect.")
        .await?;
    Ok(())
}
