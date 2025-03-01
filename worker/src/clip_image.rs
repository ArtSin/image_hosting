use std::sync::{Arc, OnceLock};

use image::{imageops::FilterType, DynamicImage};
use ndarray::{arr3, Array3, Axis};
use ort::{
    execution_providers::CUDAExecutionProvider,
    session::{builder::GraphOptimizationLevel, Session},
};
use tokio::sync::mpsc;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    batch_processing::{batch_process, log_processing_function, start_batch_process, Command},
    util::ToNdarray3,
    Embedding, Settings,
};

static MODEL: OnceLock<Session> = OnceLock::new();
static BATCH_SENDER: OnceLock<mpsc::Sender<Command<Array3<f32>, Embedding>>> = OnceLock::new();

pub fn initialize_model(settings: &Settings) -> ort::Result<()> {
    MODEL
        .set(
            Session::builder()?
                .with_execution_providers([CUDAExecutionProvider::default().build()])?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file("models/clip-ViT-B-32/model.onnx")?,
        )
        .unwrap_or_log();
    BATCH_SENDER
        .set(start_batch_process(settings, |batch| {
            log_processing_function("CLIP/Image", compute_embeddings, batch)
        }))
        .unwrap_or_log();
    Ok(())
}

fn preprocess_image(image: &DynamicImage) -> Array3<f32> {
    const SIZE: u32 = 224;

    // Resize
    let (h, w) = (image.height(), image.width());
    let (short, long) = if w <= h { (w, h) } else { (h, w) };
    let (new_short, new_long) = (
        SIZE,
        ((SIZE as f32) * (long as f32) / (short as f32)).floor() as u32,
    );
    let (new_w, new_h) = if w <= h {
        (new_short, new_long)
    } else {
        (new_long, new_short)
    };
    let mut image = image.resize_exact(new_w, new_h, FilterType::CatmullRom);

    // Center crop
    let crop_top = (((new_h - SIZE) as f32) / 2.0).round() as u32;
    let crop_left = (((new_w - SIZE) as f32) / 2.0).round() as u32;
    image = image.crop_imm(crop_left, crop_top, SIZE, SIZE);

    // To RGB array
    let arr = image.to_rgb8().into_ndarray3().mapv(|x| x as f32) / 255.0;

    // Normalize
    #[allow(clippy::excessive_precision)]
    {
        let mean: Array3<f32> = arr3(&[[[0.48145466]], [[0.4578275]], [[0.40821073]]]);
        let std: Array3<f32> = arr3(&[[[0.26862954]], [[0.26130258]], [[0.27577711]]]);
        (arr - mean) / std
    }
}

fn compute_embeddings(arrays: Vec<Array3<f32>>) -> anyhow::Result<Vec<Embedding>> {
    let session = MODEL.get().unwrap_or_log();

    let pixel_values = ndarray::stack(
        Axis(0),
        &arrays.iter().map(|x| x.view()).collect::<Vec<_>>(),
    )
    .unwrap_or_log();

    let output = session.run(ort::inputs![pixel_values]?)?;

    let res: Vec<_> = output[0]
        .try_extract_tensor::<f32>()
        .unwrap_or_log()
        .outer_iter()
        .map(|x| Embedding::from_unnormalized_array(x.into_owned()))
        .collect();
    Ok(res)
}

pub async fn process_request(image: Arc<DynamicImage>) -> Embedding {
    let array = tokio::task::spawn_blocking(move || preprocess_image(&image))
        .await
        .unwrap_or_log();
    batch_process(BATCH_SENDER.get().unwrap_or_log(), array, false).await
}
