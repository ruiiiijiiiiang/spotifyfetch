// use colored::Colorize;
use std::{
    error::Error,
    io::{self, Write},
};
use strum::EnumMessage;

pub mod api;
pub mod auth;
pub mod config;
pub mod image;

use crate::api::Api;
use crate::auth::AuthToken;
use crate::config::{Config, ItemType};
use crate::image::Image;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load();
    let access_token = AuthToken::get_valid_token().await?;
    let api = Api::new(access_token, config.time_range);

    let (track_count, artist_count) = config.get_item_count();
    let tracks = api.fetch_user_top_tracks(track_count as u32).await?;
    let artists = api.fetch_user_top_artists(artist_count as u32).await?;

    if tracks.is_empty() || artists.is_empty() {
        println!(
            "You have no Spotify listening data from the most recent {}",
            config.time_range.get_message().unwrap()
        );
        std::process::exit(0);
    }

    let (image, image_caption) = match config.image_view {
        ItemType::Track => {
            if let Some(track) = tracks.first()
                && let Ok(image) = Image::new(&track.album.images).await
            {
                let image_caption = format!("🎶 Favorite track: {}", track.format_track_display(),);
                (Some(image), Some(image_caption))
            } else {
                (None, None)
            }
        }
        ItemType::Artist => {
            if let Some(artist) = artists.first()
                && let Ok(image) = Image::new(&artist.images).await
            {
                let image_caption = format!("🎤 Favorite artist: {}", artist.name);
                (Some(image), Some(image_caption))
            } else {
                (None, None)
            }
        }
    };

    let text_lines = match config.list_view {
        ItemType::Artist => {
            let mut text_lines = vec![format!("🎤 Top {} Artists:", config.list_count)];
            for (i, artist) in artists.iter().enumerate() {
                text_lines.push(format!("  {}. {}", i + 1, artist.name));
            }
            text_lines
        }
        ItemType::Track => {
            let mut text_lines = vec![format!("🎶 Top {} Tracks:", config.list_count)];
            for (i, track) in tracks.iter().enumerate() {
                text_lines.push(format!("  {}. {}", i + 1, track.format_track_display(),));
            }
            text_lines
        }
    };

    if let Some(image) = image
        && let Some(image_caption) = image_caption
    {
        render_output(&config, &image, image_caption, text_lines)?;
    }

    std::process::exit(0);
}

fn render_output(
    config: &Config,
    image: &Image,
    image_caption: String,
    text_lines: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Your Spotify stats from the most recent {}:",
        config.time_range.get_message().unwrap()
    );

    let image_term_height = image.get_terminal_height(config.image_width.into())?;
    let text_height = text_lines.len() as u32;
    let total_height = (image_term_height + 1).max(text_height);

    // Reserve vertical space by printing enough newlines
    for _ in 0..total_height {
        println!();
    }
    // Move cursor back up to where we want to start drawing
    print!("\x1b[{}A", total_height);
    io::stdout().flush()?;

    let conf = viuer::Config {
        // width: Some(config.image_width.into()),
        height: Some(image_term_height),
        absolute_offset: false,
        restore_cursor: false,
        x: config.offset_x,
        y: config.offset_y as i16,
        ..Default::default()
    };

    // Print the image
    viuer::print_from_file(&image.path, &conf)?;
    println!("{}", image_caption);

    // Move cursor back to top of image
    print!("\x1b[{}A", image_term_height - 1);
    io::stdout().flush()?;

    // Move cursor right to position after image
    let text_column = config.image_width + config.offset_x + config.gap;

    for line in text_lines.iter() {
        print!("\x1b[{}C{}", text_column, line); // Move right and print
        print!("\x1b[1E"); // Move to beginning of next line
        io::stdout().flush()?;
    }

    // Move cursor below the image
    let lines_printed = text_lines.len() as u32;
    print!("\x1b[{}A", lines_printed);
    io::stdout().flush()?;
    print!("\x1b[{}B", total_height);
    io::stdout().flush()?;

    Ok(())
}
