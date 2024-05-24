use std::sync::OnceLock;

use ndarray::{Array2, ArrayD, ArrayViewD, Axis};
use ort::{CUDAExecutionProvider, GraphOptimizationLevel, Session};
use tokenizers::{EncodeInput, PaddingParams, Tokenizer, TruncationParams};
use tokio::sync::mpsc;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    batch_processing::{batch_process, log_processing_function, start_batch_process, Command},
    Embedding, Settings,
};

static MAIN_MODEL: OnceLock<Session> = OnceLock::new();
static DENSE_MODEL: OnceLock<Session> = OnceLock::new();
static TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();
static BATCH_SENDER: OnceLock<mpsc::Sender<Command<String, Embedding>>> = OnceLock::new();

struct PreprocessedText {
    pub input_ids: Array2<i64>,
    pub attention_mask: Array2<i64>,
}

pub fn initialize_model(settings: &Settings) -> anyhow::Result<()> {
    MAIN_MODEL
        .set(
            Session::builder()?
                .with_execution_providers([CUDAExecutionProvider::default().build()])?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file("models/clip-ViT-B-32-multilingual-v1/model.onnx")?,
        )
        .unwrap_or_log();
    // Always on CPU
    DENSE_MODEL
        .set(
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file("models/clip-ViT-B-32-multilingual-v1/dense.onnx")?,
        )
        .unwrap_or_log();
    TOKENIZER
        .set(
            Tokenizer::from_file("models/clip-ViT-B-32-multilingual-v1/tokenizer.json")
                .map(|mut x| {
                    x.with_padding(Some(PaddingParams::default()));
                    x.with_truncation(Some(TruncationParams::default()))
                        .unwrap();
                    x
                })
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .unwrap_or_log();
    BATCH_SENDER
        .set(start_batch_process(settings, |batch| {
            log_processing_function("CLIP/Text", compute_embeddings, batch)
        }))
        .unwrap_or_log();
    Ok(())
}

fn preprocess_texts<'a, T: Into<EncodeInput<'a>> + Send>(
    tokenizer: &Tokenizer,
    texts: Vec<T>,
) -> tokenizers::Result<PreprocessedText> {
    let encodings = tokenizer.encode_batch(texts, true)?;
    let sequence_length = encodings[0].get_ids().len();
    let input_ids: Vec<_> = encodings
        .iter()
        .flat_map(|a| a.get_ids().iter().map(|x| *x as i64).collect::<Vec<_>>())
        .collect();
    let attention_mask: Vec<_> = encodings
        .iter()
        .flat_map(|a| {
            a.get_attention_mask()
                .iter()
                .map(|x| *x as i64)
                .collect::<Vec<_>>()
        })
        .collect();

    Ok(PreprocessedText {
        input_ids: Array2::from_shape_vec((encodings.len(), sequence_length), input_ids)
            .unwrap_or_log(),
        attention_mask: Array2::from_shape_vec((encodings.len(), sequence_length), attention_mask)
            .unwrap_or_log(),
    })
}

fn mean_pooling(last_hidden_state: &ArrayViewD<f32>, attention_mask: Array2<i64>) -> ArrayD<f32> {
    let input_mask_expanded = attention_mask
        .insert_axis(Axis(2))
        .broadcast(last_hidden_state.dim())
        .unwrap_or_log()
        .mapv(|x| x as f32);
    (last_hidden_state * &input_mask_expanded.view()).sum_axis(Axis(1))
        / (input_mask_expanded.sum_axis(Axis(1)).mapv(|x| x.max(1e-9)))
}

fn compute_embeddings(texts: Vec<String>) -> anyhow::Result<Vec<Embedding>> {
    let session_main = MAIN_MODEL.get().unwrap_or_log();
    let session_dense = DENSE_MODEL.get().unwrap_or_log();
    let tokenizer = TOKENIZER.get().unwrap_or_log();

    let PreprocessedText {
        input_ids,
        attention_mask,
    } = preprocess_texts(tokenizer, texts).unwrap_or_log();

    let output_main = session_main.run(ort::inputs![input_ids, attention_mask.clone()]?)?;
    let res_main = mean_pooling(
        &output_main[0].try_extract_tensor::<f32>().unwrap_or_log(),
        attention_mask,
    );
    let output_dense = session_dense.run(ort::inputs![res_main]?)?;

    let res: Vec<_> = output_dense[0]
        .try_extract_tensor::<f32>()
        .unwrap_or_log()
        .outer_iter()
        .map(|x| Embedding::from_unnormalized_array(x.into_owned()))
        .collect();
    Ok(res)
}

pub async fn process_request(text: String) -> Embedding {
    batch_process(BATCH_SENDER.get().unwrap_or_log(), text, false).await
}
