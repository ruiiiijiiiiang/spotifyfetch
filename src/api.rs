use serde::Deserialize;
use std::error::Error;
use url::Url;

use crate::config::TimeRange;

pub struct Api {
    access_token: String,
    time_range: TimeRange,
}

impl Api {
    pub fn new(access_token: String, time_range: TimeRange) -> Self {
        Api {
            access_token,
            time_range,
        }
    }

    pub async fn fetch_user_top_artists(&self, limit: u32) -> Result<Vec<Artist>, Box<dyn Error>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let url = self.build_url("artists", limit);

        let top_artists: TopArtistsResponse = self.fetch_spotify_api(&url).await?;
        Ok(top_artists.items)
    }

    pub async fn fetch_user_top_tracks(&self, limit: u32) -> Result<Vec<Track>, Box<dyn Error>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let url = self.build_url("tracks", limit);

        let top_tracks: TopTracksResponse = self.fetch_spotify_api(&url).await?;
        Ok(top_tracks.items)
    }

    fn build_url(&self, endpoint: &str, limit: u32) -> String {
        let base = format!("https://api.spotify.com/v1/me/top/{}", endpoint);
        let mut url = Url::parse(&base).unwrap();
        url.query_pairs_mut()
            .append_pair("time_range", &self.time_range.to_string())
            .append_pair("limit", &limit.to_string());
        url.to_string()
    }

    async fn fetch_spotify_api<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<T, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", &self.access_token))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API error {}: {}", status, error_text).into());
        }

        Ok(response.json().await?)
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct Image {
    pub url: String,
    pub height: u32,
    pub width: u32,
}

#[derive(Deserialize, Debug)]
pub struct Artist {
    pub name: String,
    pub images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct TopArtistsResponse {
    pub items: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
pub struct SimpleArtist {
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Album {
    pub name: String,
    pub images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct Track {
    pub name: String,
    pub artists: Vec<SimpleArtist>,
    pub album: Album,
}

impl Track {
    pub fn format_track_display(&self) -> String {
        format!(
            "{} - {} ({})",
            self.name,
            self.artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            self.album.name
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct TopTracksResponse {
    items: Vec<Track>,
}
