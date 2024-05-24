use std::{io::Cursor, sync::Arc};

use common::{
    storage::{get_image_path, load_image, symlink_thumbnail},
    OnUploadMessage, ELASTICSEARCH_INDEX,
};
use elasticsearch::CreateParts;
use exif::{In, Tag};
use image::{
    imageops::{self, FilterType},
    DynamicImage,
};
use serde_json::json;
use tracing_unwrap::ResultExt;

use crate::{clip_image, ELASTICSEARCH};

const MAX_WIDTH: u32 = 800;
const MAX_HEIGHT: u32 = 600;

async fn create_thumbnail(
    message: Arc<OnUploadMessage>,
    image: Arc<DynamicImage>,
    image_buf: Vec<u8>,
) -> Result<(), ()> {
    if image.width() <= MAX_WIDTH && image.height() <= MAX_HEIGHT {
        symlink_thumbnail(message.id, &message.format)
            .await
            .map_err(|e| tracing::error!("Can't symlink thumbnail: {e}"))?;
    } else {
        tokio::task::spawn_blocking(move || {
            let mut cursor = Cursor::new(image_buf);
            let exif_reader = exif::Reader::new();
            let orientation = exif_reader
                .read_from_container(&mut cursor)
                .map(|exif| match exif.get_field(Tag::Orientation, In::PRIMARY) {
                    Some(orientation) => match orientation.value.get_uint(0) {
                        Some(v @ 1..=8) => v,
                        _ => 1,
                    },
                    None => 1,
                })
                .unwrap_or(1);

            let mut thumbnail = image
                .resize(MAX_WIDTH, MAX_HEIGHT, FilterType::CatmullRom)
                .to_rgb8();
            if orientation == 2 {
                thumbnail = imageops::flip_horizontal(&thumbnail);
            } else if orientation == 3 {
                thumbnail = imageops::rotate180(&thumbnail);
            } else if orientation == 4 {
                thumbnail = imageops::flip_horizontal(&thumbnail);
            } else if orientation == 5 {
                thumbnail = imageops::rotate90(&thumbnail);
                thumbnail = imageops::flip_horizontal(&thumbnail);
            } else if orientation == 6 {
                thumbnail = imageops::rotate90(&thumbnail);
            } else if orientation == 7 {
                thumbnail = imageops::rotate270(&thumbnail);
                thumbnail = imageops::flip_horizontal(&thumbnail);
            } else if orientation == 8 {
                thumbnail = imageops::rotate270(&thumbnail);
            }

            let path = get_image_path(message.id, &message.format, true);
            thumbnail
                .save(path)
                .map_err(|e| tracing::error!("Can't save thumbnail: {e}"))
        })
        .await
        .unwrap_or_log()?;
    }
    Ok(())
}

async fn add_to_elasticsearch(
    message: &OnUploadMessage,
    image: Arc<DynamicImage>,
) -> Result<(), ()> {
    let embedding = clip_image::process_request(image).await;

    ELASTICSEARCH
        .get()
        .unwrap()
        .create(CreateParts::IndexId(
            ELASTICSEARCH_INDEX,
            &message.id.to_string(),
        ))
        .body(json!({"title": message.title, "embedding": embedding.embedding}))
        .send()
        .await
        .map_err(|e| tracing::error!("Can't add to Elasticsearch: {e}"))?;
    Ok(())
}

pub async fn process_request(message: OnUploadMessage) -> Result<(), ()> {
    let image_buf = load_image(get_image_path(message.id, &message.format, false))
        .await
        .map_err(|e| tracing::error!("Can't load image: {e}"))?;
    let image = Arc::new(
        image::load_from_memory(&image_buf)
            .map_err(|e| tracing::error!("Can't read image: {e}"))?,
    );

    let message = Arc::new(message);
    let image_ = Arc::clone(&image);
    let (res_1, res_2) = tokio::join!(
        create_thumbnail(Arc::clone(&message), image, image_buf),
        add_to_elasticsearch(&message, image_)
    );
    if res_1.is_err() || res_2.is_err() {
        return Err(());
    }
    Ok(())
}
