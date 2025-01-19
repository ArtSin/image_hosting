use amqprs::{channel::BasicPublishArguments, BasicProperties};
use common::{SearchMessage, SearchResponse, ELASTICSEARCH_INDEX};
use elasticsearch::SearchParts;
use serde_json::{json, Value};
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{clip_text, Embedding, ELASTICSEARCH, RABBITMQ_CHANNEL};

const RESULTS_PER_PAGE: i64 = 6;
const KNN_PAGES: i64 = 20;
const KNN_K: i64 = RESULTS_PER_PAGE * KNN_PAGES;

async fn search_in_elasticsearch(
    message: &SearchMessage,
    embedding: Embedding,
) -> Result<SearchResponse, ()> {
    let request_body = json!({
        "query": {
            "simple_query_string" : {
                "query": message.query_text,
                "fields": ["title"]
            }
        },
        "knn": {
            "field": "embedding",
            "query_vector": embedding.embedding,
            "k": KNN_K
        },
        "_source": false
    });

    let res = ELASTICSEARCH
        .get()
        .unwrap()
        .search(SearchParts::Index(&[ELASTICSEARCH_INDEX]))
        .from(message.page * RESULTS_PER_PAGE)
        .size(RESULTS_PER_PAGE + 1)
        .body(request_body)
        .send()
        .await
        .map_err(|e| tracing::error!("Can't search in Elasticsearch: {e}"))?
        .json::<Value>()
        .await
        .unwrap_or_log();

    let mut ids: Vec<_> = res["hits"]["hits"]
        .as_array()
        .unwrap_or_log()
        .iter()
        .map(|val| val["_id"].as_str().unwrap().parse().unwrap())
        .collect();
    let mut last_page = true;
    if ids.len() == (RESULTS_PER_PAGE + 1) as usize {
        last_page = false;
        ids.pop();
    }

    Ok(SearchResponse { ids, last_page })
}

pub async fn process_request(
    message: SearchMessage,
    reply_to: Option<&String>,
    correlation_id: Option<&String>,
) -> Result<(), ()> {
    let embedding = clip_text::process_request(message.query_text.clone()).await;
    let response =
        serde_json::to_vec(&search_in_elasticsearch(&message, embedding).await?).unwrap_or_log();

    let props = BasicProperties::default()
        .with_persistence(true)
        .with_correlation_id(correlation_id.unwrap_or_log())
        .finish();
    let args = BasicPublishArguments::default()
        .routing_key(reply_to.unwrap_or_log().to_owned())
        .finish();
    RABBITMQ_CHANNEL
        .read()
        .await
        .as_ref()
        .unwrap()
        .basic_publish(props, response, args)
        .await
        .map_err(|e| tracing::error!("Can't send search results: {e}"))?;
    Ok(())
}
