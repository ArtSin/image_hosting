use common::ELASTICSEARCH_INDEX;
use elasticsearch::{
    http::StatusCode,
    indices::{IndicesCreateParts, IndicesExistsParts},
    Elasticsearch,
};
use serde_json::json;

pub async fn create_index(es_client: &Elasticsearch) -> Result<(), elasticsearch::Error> {
    // Check if index exists
    if es_client
        .indices()
        .exists(IndicesExistsParts::Index(&[ELASTICSEARCH_INDEX]))
        .send()
        .await?
        .status_code()
        == StatusCode::OK
    {
        return Ok(());
    }

    // Create index and set mapping
    es_client
        .indices()
        .create(IndicesCreateParts::Index(ELASTICSEARCH_INDEX))
        .body(json!({
            "settings": {
                "index": {
                    "analysis": {
                        "filter": {
                            "english_stemmer": {
                                "type": "stemmer",
                                "name": "english"
                            },
                            "russian_stemmer": {
                                "type": "stemmer",
                                "name": "russian"
                            },
                            "english_stop": {
                                "type": "stop",
                                "stopwords": "_english_"
                            },
                            "russian_stop": {
                                "type": "stop",
                                "stopwords": "_russian_"
                            }
                        },
                        "analyzer": {
                            "en_ru_analyzer": {
                                "tokenizer": "standard",
                                "filter": [
                                    "lowercase",
                                    "english_stemmer",
                                    "russian_stemmer",
                                    "english_stop",
                                    "russian_stop"
                                ]
                            }
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "title": {
                        "type": "text",
                        "analyzer": "en_ru_analyzer"
                    },
                    "embedding": {
                        "type": "dense_vector",
                        "dims": 512,
                        "index": true,
                        "similarity": "dot_product"
                    }
                }
            }
        }))
        .send()
        .await?
        .error_for_status_code()?;
    Ok(())
}
