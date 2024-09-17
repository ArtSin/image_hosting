use std::{
    fs::Metadata,
    io::Cursor,
    path::{Path, PathBuf},
};

const STORAGE_PATH: &str = "storage";
const IMAGES_PATH: &str = "images";
const THUMBNAILS_PATH: &str = "thumbnails";

pub async fn create_folders() -> std::io::Result<()> {
    let mut path = PathBuf::from(STORAGE_PATH);
    path.push(IMAGES_PATH);
    tokio::fs::create_dir_all(&path).await?;
    path.pop();
    path.push(THUMBNAILS_PATH);
    tokio::fs::create_dir_all(path).await
}

pub fn get_image_format(
    image: &Vec<u8>,
    image_extensions: &[&'static str],
) -> Result<&'static str, String> {
    let reader = image::ImageReader::new(Cursor::new(image))
        .with_guessed_format()
        .unwrap();
    let format = reader
        .format()
        .ok_or_else(|| "unsupported_image_format".to_owned())?;
    let format = format.extensions_str()[0];
    if !image_extensions.contains(&format) {
        return Err("unsupported_image_format".to_owned());
    }
    Ok(format)
}

pub fn get_image_path(id: i64, format: &str, thumbnail: bool) -> PathBuf {
    let mut path = PathBuf::from(STORAGE_PATH);
    if thumbnail {
        path.push(THUMBNAILS_PATH);
    } else {
        path.push(IMAGES_PATH);
    }
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

pub async fn symlink_thumbnail(id: i64, format: &str) -> Result<(), String> {
    let mut src = PathBuf::from("..");
    src.push(IMAGES_PATH);
    src.push(format!("{id}.{format}"));

    let dst = get_image_path(id, format, true);
    tokio::fs::symlink(src, dst)
        .await
        .map_err(|_| "storage_error".to_owned())
}
