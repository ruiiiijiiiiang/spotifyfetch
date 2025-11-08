use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize, Debug)]
pub struct Artist {
    pub name: String,
    pub genres: Vec<String>,
    pub popularity: u32,
    pub images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct TopArtistsResponse {
    pub items: Vec<Artist>,
}

pub async fn fetch_user_top_artists(
    access_token: &str,
    time_range: &str,
    limit: u32,
) -> Result<Vec<Artist>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let url = format!(
        "https://api.spotify.com/v1/me/top/artists?time_range={}&limit={}",
        time_range, limit
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("API error {}: {}", status, error_text).into());
    }

    let top_artists: TopArtistsResponse = response.json().await?;
    Ok(top_artists.items)
}

#[derive(Deserialize, Debug)]
pub struct Track {
    pub name: String,
    pub artists: Vec<SimpleArtist>,
    pub album: Album,
    pub popularity: u32,
}

#[derive(Deserialize, Debug)]
pub struct SimpleArtist {
    pub name: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Image {
    pub url: String,
    pub height: u32,
    pub width: u32,
}

#[derive(Deserialize, Debug)]
pub struct Album {
    pub name: String,
    pub images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct TopTracksResponse {
    items: Vec<Track>,
}

pub async fn fetch_user_top_tracks(
    access_token: &str,
    time_range: &str,
    limit: u32,
) -> Result<Vec<Track>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let url = format!(
        "https://api.spotify.com/v1/me/top/tracks?time_range={}&limit={}",
        time_range, limit
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("API error {}: {}", status, error_text).into());
    }

    let top_tracks: TopTracksResponse = response.json().await?;
    Ok(top_tracks.items)
}

pub fn format_track_artist(track: &Track) -> String {
    track
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
