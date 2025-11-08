use colored::Colorize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};
use strum::EnumMessage;

pub mod api;
pub mod auth;
pub mod config;
pub mod image;

use crate::auth::AuthToken;
use crate::config::{Config, ItemType};
use crate::image::{download_image, get_best_image_url, get_image_terminal_height};
use crate::{
    api::{fetch_user_top_artists, fetch_user_top_tracks, format_track_artist},
    config::TimeRange,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load();

    let access_token = AuthToken::get_valid_token().await?;

    let (track_count, artist_count) = match (config.image_view, config.list_view) {
        (ItemType::Track, ItemType::Artist) => (1, config.list_count),
        (ItemType::Track, ItemType::Track) => (config.list_count, 0),
        (ItemType::Artist, ItemType::Track) => (config.list_count, 1),
        (ItemType::Artist, ItemType::Artist) => (0, config.list_count),
    };
    let tracks = fetch_user_top_tracks(
        &access_token,
        &config.time_range.to_string(),
        track_count as u32,
    )
    .await?;
    let artists = fetch_user_top_artists(
        &access_token,
        &config.time_range.to_string(),
        artist_count as u32,
    )
    .await?;

    if tracks.is_empty() || artists.is_empty() {
        println!(
            "You have no Spotify listening data from the most recent {}",
            config.time_range.get_message().unwrap()
        );
    }

    let (image_path, image_caption) = match config.image_view {
        ItemType::Track => {
            if let Some(track) = tracks.first()
                && let Some(image_url) = get_best_image_url(&track.album.images)
                && let Ok(image_path) = download_image(&image_url).await
            {
                let image_caption = format!(
                    "🎶 Favorite track: {} - {} ({})",
                    track.name,
                    format_track_artist(track),
                    track.album.name
                );
                (Some(image_path), Some(image_caption))
            } else {
                (None, None)
            }
        }
        ItemType::Artist => {
            if let Some(artist) = artists.first()
                && let Some(image_url) = get_best_image_url(&artist.images)
                && let Ok(image_path) = download_image(&image_url).await
            {
                let image_caption = format!("🎤 Favorite artist: {}", artist.name);
                (Some(image_path), Some(image_caption))
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
                text_lines.push(format!(
                    "  {}. {} - {} ({})",
                    i + 1,
                    track.name.green(),
                    format_track_artist(track),
                    track.album.name
                ));
            }
            text_lines
        }
    };

    if image_path.is_some() && image_caption.is_some() {
        render_output(
            config.offset_x,
            config.offset_y,
            config.gap,
            config.time_range,
            &image_path.unwrap(),
            image_caption.unwrap(),
            text_lines,
            config.image_width as u32,
        )?;
    }

    std::process::exit(0);
}

pub fn render_output(
    offset_x: u16,
    offset_y: u16,
    gap: u32,
    time_range: TimeRange,
    image_path: &Path,
    image_caption: String,
    text_lines: Vec<String>,
    image_width: u32,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Your Spotify stats from the most recent {}:",
        time_range.get_message().unwrap()
    );

    let image_term_height = get_image_terminal_height(image_path, image_width)?;
    let text_height = text_lines.len() as u32;
    let total_height = image_term_height.max(text_height);

    for _ in 0..total_height {
        println!();
    }
    print!("\x1b[{}A", total_height);
    io::stdout().flush()?;

    let conf = viuer::Config {
        width: Some(image_width),
        absolute_offset: false,
        restore_cursor: true,
        x: offset_x,
        y: offset_y as i16,
        ..Default::default()
    };
    viuer::print_from_file(image_path, &conf)?;

    let text_column = image_width + (offset_x as u32) + gap;

    print!("\x1b[{}C{}", text_column, image_caption); // Move right and print
    print!("\x1b[2E"); // Move to beginning of next line
    io::stdout().flush()?;

    for line in text_lines.iter() {
        print!("\x1b[{}C{}", text_column, line);
        print!("\x1b[1E");
        io::stdout().flush()?;
    }

    let lines_printed = text_lines.len() as u32 + 3;
    if image_term_height > lines_printed {
        print!("\x1b[{}B", image_term_height - lines_printed);
    }
    print!("\r\n");
    io::stdout().flush()?;

    Ok(())
}
