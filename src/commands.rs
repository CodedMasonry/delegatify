use crate::database::{db_add_user, db_get_user_permission, db_remove_user, db_user_exists};
use crate::spotify::{fetch_queue, fetch_track, StandardItem};
use crate::{format_delta, is_frozen, spotify, Context, Error};
use anyhow::Context as _;
use poise::serenity_prelude::{
    self as serenity, ButtonStyle, Colour, CreateActionRow, CreateButton, CreateEmbed,
    CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponse, Timestamp, UserId,
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

/*

Playback Commands

*/

/// Check the current playback
#[poise::command(slash_command, user_cooldown = 10, category = "Playback")]
pub async fn current(ctx: Context<'_>) -> Result<(), Error> {
    run_current(ctx).await
}

/// Check the queue
#[poise::command(slash_command, user_cooldown = 10, category = "Playback")]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(());
        }
    };

    // The current playing song
    let current = match client.current_playing(None, None::<Vec<_>>).await? {
        Some(v) => StandardItem::parse(v.item.unwrap()),
        None => {
            let embed = current_no_playback(CreateEmbed::default()).await;
            ctx.send(CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
    };
    drop(lock);

    // The queue
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

    if queue.len() == 0 {
        ctx.say("Nothings in the queue.").await?;
        return Ok(());
    }

    let embed = CreateEmbed::new()
        .colour(Colour::DARK_GREEN)
        .author(
            CreateEmbedAuthor::new(current.get_title())
            .url(current.url)
            .icon_url("https://storage.googleapis.com/pr-newsroom-wp/1/2023/05/Spotify_Primary_Logo_RGB_Green.png"),
        )
        .title("Current Queue")
        .description("The next five songs that are in the queue.")
        .thumbnail(current.image)
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Delegatify"))
        .description(format!("{}\n**...**", queue.join("\n\n")));

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Add a song to the queue
#[poise::command(
    slash_command,
    user_cooldown = 60,
    global_cooldown = 30,
    category = "Playback"
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Either the URL or search query"]
    #[max_length = 512]
    input: String,
) -> Result<(), Error> {
    if !allow_playback(ctx, 1).await? {
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
    let title = track.get_title();
    let embed = CreateEmbed::new()
        .colour(Colour::DARK_GREEN)
        .author(CreateEmbedAuthor::new("Added Song To Queue"))
        .title(title.clone())
        .thumbnail(track.image)
        .field(
            "Length",
            format!("{}s", format_delta(track.duration)),
            false,
        )
        .timestamp(Timestamp::now())
        .footer(
            CreateEmbedFooter::new(format!("Requested by {}", ctx.author().name))
                .icon_url(ctx.author().avatar_url().unwrap_or_default()),
        );

    ctx.send(CreateReply::default().embed(embed)).await?;

    // Just some logging
    info!(
        "{} added {} to the queue",
        user_to_id(ctx.author().id).await,
        title,
    );
    Ok(())
}

/// Play the previous track
#[poise::command(
    slash_command,
    user_cooldown = 60,
    global_cooldown = 30,
    category = "Playback"
)]
pub async fn previous(ctx: Context<'_>) -> Result<(), Error> {
    if !allow_playback(ctx, 1).await? {
        return Ok(());
    }

    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(());
        }
    };

    client.previous_track(None).await?;
    drop(lock);

    run_current(ctx).await?;

    // Just some logging
    info!(
        "{} skipped to the next song",
        user_to_id(ctx.author().id).await
    );
    Ok(())
}

/// Play the next track
#[poise::command(
    slash_command,
    user_cooldown = 60,
    global_cooldown = 30,
    category = "Playback"
)]
pub async fn next(ctx: Context<'_>) -> Result<(), Error> {
    if !allow_playback(ctx, 1).await? {
        return Ok(());
    }

    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(());
        }
    };

    client.next_track(None).await?;
    drop(lock);

    run_current(ctx).await?;

    // Just some logging
    info!(
        "{} skipped to the previous song",
        user_to_id(ctx.author().id).await
    );
    Ok(())
}

/*

Utilities Commands

*/

/// Switch the state of freeze
#[poise::command(slash_command, owners_only, category = "Utilities")]
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

/// Allow a user with specific permissions
#[poise::command(slash_command, owners_only, category = "Utilities")]
pub async fn add_user(
    ctx: Context<'_>,
    #[description = "Person to add"] user: serenity::User,
    #[description = "Permission level to set for user; default to basic (1)"] level: Option<i16>,
) -> Result<(), Error> {
    let id = user_to_id(user.clone().id).await;

    if db_user_exists(&ctx.data().pool, id).await? {
        ctx.say("User already added").await?;
        return Ok(());
    }

    db_add_user(&ctx.data().pool, id, level).await?;
    ctx.say("Successfully added user").await?;
    Ok(())
}

/// Allow a user with specific permissions
#[poise::command(slash_command, owners_only, category = "Utilities")]
pub async fn remove_user(
    ctx: Context<'_>,
    #[description = "Person to remove"] user: serenity::User,
) -> Result<(), Error> {
    let id = user_to_id(user.clone().id).await;

    if !db_user_exists(&ctx.data().pool, id).await? {
        ctx.say("User isn't in database").await?;
        return Ok(());
    }

    db_remove_user(&ctx.data().pool, id).await?;
    ctx.say("Successfully removed user").await?;
    Ok(())
}

/// Authenticates the application with specified token
#[poise::command(slash_command, owners_only, category = "Utilities")]
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
            .field("Authenticate Button", "This is the button you click when you have the code. It will ask you to input the code, and then you are good to go.", false)
            .footer(CreateEmbedFooter::new(format!("Version: {}", env!("CARGO_PKG_VERSION")))))
            .components(components)
    };

    ctx.send(reply).await?;

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(std::time::Duration::from_secs(120))
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
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

/*
        End Commands; Start libs
*/

/// Inner command of current
async fn run_current(ctx: Context<'_>) -> Result<(), Error> {
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
        Some(item) => current_playback(&playback, item.clone(), embed).await,
        None => current_no_playback(embed).await,
    };

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// If there is a currently playing song
async fn current_playback(
    playback: &CurrentPlaybackContext,
    item: PlayableItem,
    embed: CreateEmbed,
) -> CreateEmbed {
    let item = StandardItem::parse(item);

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
        .footer(CreateEmbedFooter::new(format!(
            "Playing on {}",
            playback.device.name
        )))
        .author(CreateEmbedAuthor::new("Currently Playing..."))
        .title(item.get_title())
        .thumbnail(item.image)
        .field("Time", duration, false)
        .field("Shuffle", shuffle, true)
        .field("Repeat", repeat, true)
}

/// If there is no song playing
async fn current_no_playback(embed: CreateEmbed) -> CreateEmbed {
    // Create Embed
    embed
        .color(Colour::DARK_RED)
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Delegatify"))
        .title("Nothing Playing")
        .description("Nothing is currently being played ")
}

/// Parse a URL for TrackId
async fn play_url<'a>(url: &'a str) -> Result<TrackId<'a>, IdError> {
    let id = url.split('/').last().unwrap().split('?').next().unwrap();
    TrackId::from_id(id)
}

/// Use search to confirm song, return TrackId
async fn play_search(ctx: Context<'_>, input: String) -> Result<TrackId<'_>, Error> {
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
        .search(&input, SearchType::Track, None, None, Some(5), None)
        .await?;
    // Free client
    drop(lock);

    // Make the data into standard items
    let mut data = Vec::new();
    if let SearchResult::Tracks(page) = search_result {
        for item in page.items {
            let item = StandardItem::parse(PlayableItem::Track(item));
            // Ignore already known tracks with the same exact title
            if !data
                .iter()
                .any(|existing: &StandardItem<'_>| existing.get_title() == item.get_title())
            {
                data.push(item)
            }
        }
    } else {
        panic!("Not Possible");
    }

    if data.len() == 0 {
        return Err("No results were found".into());
    }

    // Make a reply
    let reply = {
        let mut components = vec![];

        // Add buttons so custom id is equal to index; allows accsesing data via index
        // Take only 3 songs at most; there's guaranteed to be at least 1
        for (index, song) in data.iter().enumerate().take(3) {
            let style = if index == 0 {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            };
            components.push(CreateActionRow::Buttons(vec![CreateButton::new(
                index.to_string(),
            )
            .label(song.get_title())
            .style(style)]));
        }

        // Make cancel last
        components.push(CreateActionRow::Buttons(vec![CreateButton::new("cancel")
            .label("Cancel")
            .style(ButtonStyle::Danger)]));

        // Create the reply
        poise::CreateReply::default()
            .content("Choose A Song To Play")
            .components(components)
    };
    ctx.send(reply).await.context("Failed to send message")?;

    // Sort component interactions; Trys to convert id to int to classify it as s button
    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
        .timeout(std::time::Duration::from_secs(120))
        .author_id(ctx.author().id)
        .filter(|v| v.data.custom_id == "cancel" || v.data.custom_id.parse::<u8>().is_ok())
        .await
    {
        // Tell discord we got the interaction
        mci.create_response(ctx.http(), CreateInteractionResponse::Acknowledge)
            .await?;
        match mci.data.custom_id.as_str() {
            // If the button is cancel
            "cancel" => {
                return Err("Cancelled Interaction".into());
            }
            // If it is another item
            id => {
                let parsed = id.parse::<usize>()?;
                return Ok(data[parsed].get_track_id().unwrap().clone_static());
            }
        }
    }

    // If the interaction timed out
    Err("No interaction".into())
}

/// Checks for whether a playback command should run
async fn allow_playback(ctx: Context<'_>, min_level: i16) -> Result<bool, Error> {
    if is_frozen(ctx).await {
        ctx.say("Playback changes are frozen").await?;
        return Ok(false);
    }
    if !is_active(ctx).await? {
        ctx.say("Nothing Playing; can't modify playback.").await?;
        return Ok(false);
    }
    if is_owner(ctx).await {
        return Ok(true);
    }

    is_allowed(ctx, min_level).await
}

/// Returns whether playback is running or not
async fn is_active(ctx: Context<'_>) -> Result<bool, Error> {
    // Lock Client to get response
    let lock = ctx.data().spotify.read().await;
    let client = match &*lock {
        Some(v) => v,
        None => {
            error_unauthorized(ctx).await?;
            return Ok(false);
        }
    };

    match client.current_playing(None, None::<Vec<_>>).await? {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

/// Returns whether the user is authorised or not
async fn is_allowed(ctx: Context<'_>, min_level: i16) -> Result<bool, Error> {
    let id = user_to_id(ctx.author().id).await;
    match db_get_user_permission(&ctx.data().pool, id).await? {
        Some(level) => {
            if level < min_level {
                ctx.say("You don't have permission to run this command")
                    .await?;
                return Ok(false);
            }
        }
        None => {
            ctx.say("You don't have permission to run this command")
                .await?;
            return Ok(false);
        }
    }

    Ok(true)
}

/// Checks if user is an owner
async fn is_owner(ctx: Context<'_>) -> bool {
    ctx.framework().options.owners.contains(&ctx.author().id)
}

/// Converts a UserId to i64
async fn user_to_id(user: UserId) -> i64 {
    user.to_string().parse::<i64>().unwrap()
}

/// Error 401 response for discord
pub async fn error_unauthorized(ctx: Context<'_>) -> Result<(), Error> {
    ctx.reply("The application isn't authenticated.\nrun '/authenticate' to connect.")
        .await?;
    Ok(())
}
