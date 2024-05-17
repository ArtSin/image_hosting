#![cfg(feature = "ssr")]

use std::{
    fs::Metadata,
    io::Cursor,
    path::{Path, PathBuf},
};

use crate::image::IMAGE_EXTENSIONS;

const STORAGE_PATH: &str = "storage";

pub fn get_image_format(image: &Vec<u8>) -> Result<&'static str, String> {
    let reader = image::io::Reader::new(Cursor::new(image))
        .with_guessed_format()
        .unwrap();
    let format = reader
        .format()
        .ok_or_else(|| "unsupported_image_format".to_owned())?;
    let format = format.extensions_str()[0];
    if !IMAGE_EXTENSIONS.contains(&format) {
        return Err("unsupported_image_format".to_owned());
    }
    Ok(format)
}

pub fn get_image_path(id: i64, format: &str) -> PathBuf {
    let mut path = PathBuf::from(STORAGE_PATH);
    path.push("images");
    path.push(format!("{id}.{format}"));
    path
}

pub async fn get_image_metadata(path: impl AsRef<Path>) -> Result<Metadata, String> {
    tokio::fs::metadata(path)
        .await
        .map_err(|_| "storage_error".to_owned())
}

pub async fn load_image(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    tokio::fs::read(path)
        .await
        .map_err(|_| "storage_error".to_owned())
}

pub async fn store_image(path: impl AsRef<Path>, image: Vec<u8>) -> Result<(), String> {
    tokio::fs::write(path, image)
        .await
        .map_err(|_| "storage_error".to_owned())
}
