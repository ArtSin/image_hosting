use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const IMAGE_EXTENSIONS: [&'static str; 5] = ["jpg", "jpeg", "png", "gif", "webp"];
pub const IMAGE_MIME: [&'static str; 5] = [
    "image/jpeg",
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
];
pub const IMAGE_ACCEPT_EXT_MIME: &'static str =
    ".jpg,.jpeg,.png,.gif,.webp,image/jpeg,image/png,image/gif,image/webp";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    pub id: i64,
    pub format: String,
    pub title: String,
    pub author: i64,
    #[serde(with = "chrono::serde::ts_microseconds")]
    pub timestamp: DateTime<Utc>,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            id: -1,
            format: String::new(),
            title: String::new(),
            author: -1,
            timestamp: DateTime::<Utc>::MIN_UTC,
        }
    }
}
