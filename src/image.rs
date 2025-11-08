use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use viuer::{Config, print_from_file};

use crate::api::Image;

pub fn get_image_terminal_height(
    image_path: &Path,
    width_columns: u32,
) -> Result<u32, Box<dyn Error>> {
    let img = image::open(image_path)?;
    let (img_width, img_height) = img.dimensions();

    // Each terminal row is roughly twice as tall as it is wide
    let aspect_ratio = img_height as f32 / img_width as f32;
    let term_height = (width_columns as f32 * aspect_ratio / 2.0).ceil() as u32;

    Ok(term_height)
}

fn get_image_cache_dir() -> Result<PathBuf, Box<dyn Error>> {
    let mut path = dirs::cache_dir().ok_or("Could not find cache directory")?;
    path.push("spotifyfetch");
    path.push("images");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn hash_url(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let result = hasher.finalize();
    format!("{:x}.jpg", result)
}

pub async fn download_image(url: &str) -> Result<PathBuf, Box<dyn Error>> {
    let cache_dir = get_image_cache_dir()?;
    let filename = hash_url(url);
    let file_path = cache_dir.join(&filename);

    if file_path.exists() {
        return Ok(file_path);
    }

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to download image: {}", response.status()).into());
    }

    let bytes = response.bytes().await?;
    fs::write(&file_path, bytes)?;

    Ok(file_path)
}

pub fn display_image(path: &Path, width: Option<u32>) -> Result<(), Box<dyn Error>> {
    let conf = Config {
        width,
        absolute_offset: false,
        ..Default::default()
    };

    print_from_file(path, &conf)?;
    Ok(())
}

pub fn get_best_image_url(images: &[Image]) -> Option<String> {
    images
        .iter()
        .max_by_key(|img| img.width * img.height)
        .map(|img| img.url.clone())
}
