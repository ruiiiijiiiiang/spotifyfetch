use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumMessage};
use validator::Validate;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Validate)]
pub struct Config {
    pub offset_x: u16,
    pub offset_y: u16,
    pub gap: u16,
    pub image_view: ItemType,
    #[validate(range(min = 25, max = 40))]
    pub image_width: u16,
    pub list_view: ItemType,
    #[validate(range(min = 1, max = 20))]
    pub list_count: u16,
    pub time_range: TimeRange,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            offset_x: 2,
            offset_y: 0,
            gap: 5,
            image_view: ItemType::Track,
            image_width: 30,
            list_view: ItemType::Artist,
            list_count: 10,
            time_range: TimeRange::Medium,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config: Self = match confy::load("spotifyfetch", "config") {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Failed to load config: {}", err);
                Config::default()
            }
        };
        match config.validate() {
            Ok(()) => config,
            Err(err) => {
                eprintln!("Invalid config: {}", err);
                Config::default()
            }
        }
    }

    pub fn get_item_count(&self) -> (u16, u16) {
        match (self.image_view, self.list_view) {
            (ItemType::Track, ItemType::Artist) => (1, self.list_count),
            (ItemType::Track, ItemType::Track) => (self.list_count, 0),
            (ItemType::Artist, ItemType::Track) => (self.list_count, 1),
            (ItemType::Artist, ItemType::Artist) => (0, self.list_count),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ItemType {
    Artist,
    Track,
}

#[derive(Display, Debug, Clone, Copy, Deserialize, Serialize, EnumMessage)]
pub enum TimeRange {
    #[strum(to_string = "short_term", message = "4 weeks")]
    Short,
    #[strum(to_string = "medium_term", message = "6 months")]
    Medium,
    #[strum(to_string = "long_term", message = "12 months")]
    Long,
}
