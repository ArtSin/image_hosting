use serde::{Deserialize, Serialize};

pub mod storage;

pub const ELASTICSEARCH_INDEX: &str = "image_hosting";
pub const RABBITMQ_QUEUE_NAME: &str = "image_hosting_queue";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerMessage {
    OnUpload(OnUploadMessage),
    Search(SearchMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnUploadMessage {
    pub id: i64,
    pub format: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMessage {
    pub query_text: String,
    pub page: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub ids: Vec<i64>,
    pub last_page: bool,
}
