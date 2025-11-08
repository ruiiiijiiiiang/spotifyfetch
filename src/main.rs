use colored::Colorize;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
};

pub mod api;
pub mod auth;
pub mod config;
pub mod image;

use crate::api::{fetch_user_top_artists, fetch_user_top_tracks, format_track_artist};
use crate::auth::AuthToken;
use crate::config::{Config, ItemType};
use crate::image::{download_image, get_best_image_url, get_image_terminal_height};

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

    let (image_path, image_caption) = match config.image_view {
        ItemType::Track => {
            if let Some(track) = tracks.first()
                && let Some(image_url) = get_best_image_url(&track.album.images)
                && let Ok(image_path) = download_image(&image_url).await
            {
                let image_caption = format!(
                    "Favorite track: {} - {} ({})",
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
                let image_caption = format!("Favorite artist: {}", artist.name);
                (Some(image_path), Some(image_caption))
            } else {
                (None, None)
            }
        }
    };

    let text_lines = match config.list_view {
        ItemType::Artist => {
            let mut text_lines = vec![format!("🎤 Top 10 Artists:")];
            for (i, artist) in artists.iter().enumerate() {
                text_lines.push(format!("  {}. {}", i + 1, artist.name));
            }
            text_lines
        }
        ItemType::Track => {
            let mut text_lines = vec![format!("🎵 Top 10 Track:")];
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
        render_layout(
            config.offset.x,
            config.offset.y,
            &image_path.unwrap(),
            image_caption.unwrap(),
            text_lines,
            config.image_width as u32,
        )?;
    }

    std::process::exit(0);
}

pub fn render_layout(
    offset_x: u16,
    offset_y: u16,
    image_path: &Path,
    image_caption: String,
    text_lines: Vec<String>,
    image_width: u32,
) -> Result<(), Box<dyn Error>> {
    let image_term_height = get_image_terminal_height(image_path, image_width)?;
    let text_height = text_lines.len() as u32;
    let total_height = image_term_height.max(text_height);

    // Reserve vertical space by printing enough newlines
    for _ in 0..total_height + 1 {
        println!();
    }
    // Move cursor back up to where we want to start drawing
    print!("\x1b[{}A", total_height);
    io::stdout().flush()?;

    let conf = viuer::Config {
        width: Some(image_width),
        absolute_offset: false,
        restore_cursor: false,
        x: offset_x,
        y: offset_y as i16,
        ..Default::default()
    };

    // Print the image
    viuer::print_from_file(image_path, &conf)?;
    println!("{}", image_caption);

    // Move cursor back to top of image
    print!("\x1b[{}A", image_term_height + 1);
    io::stdout().flush()?;

    // Move cursor right to position after image
    let text_column = image_width + 3;

    for line in text_lines.iter() {
        print!("\x1b[{}C{}", text_column, line); // Move right and print
        print!("\x1b[1E"); // Move to beginning of next line
        io::stdout().flush()?;
    }

    // Move cursor below the image
    let lines_printed = text_lines.len() as u32;
    if image_term_height > lines_printed {
        print!("\x1b[{}B", image_term_height - lines_printed);
    }
    print!("\r\n");
    io::stdout().flush()?;

    Ok(())
}
