mod batch_processing;
mod clip_image;
mod clip_text;
mod create_index;
mod on_upload;
mod search;
mod util;

use std::{sync::OnceLock, time::Duration};

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicNackArguments, Channel,
        QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use async_trait::async_trait;
use clap::Parser;
use common::{WorkerMessage, RABBITMQ_QUEUE_NAME};
use elasticsearch::{
    auth::Credentials,
    http::{
        transport::{SingleNodeConnectionPool, TransportBuilder},
        Url,
    },
    Elasticsearch,
};
use ndarray::{Array, ArrayD, Dimension};
use serde::Serialize;
use tokio::{
    signal,
    sync::{oneshot, RwLock},
};
use tracing_subscriber::{
    filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use tracing_unwrap::{OptionExt, ResultExt};

use crate::create_index::create_index;

static RABBITMQ_CHANNEL: RwLock<Option<Channel>> = RwLock::const_new(None);
static ELASTICSEARCH: OnceLock<Elasticsearch> = OnceLock::new();

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Settings {
    #[arg(long, default_value_t = 16)]
    batch_size: usize,
    #[arg(long, default_value_t = 100)]
    max_delay_ms: u64,
}

struct RabbitMQSettings {
    host: String,
    port: u16,
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Embedding {
    pub embedding: Vec<f32>,
}

impl Embedding {
    pub fn normalize<D: Dimension>(arr: Array<f32, D>) -> Array<f32, D> {
        const NORMALIZE_EPS: f32 = 1e-12;

        let norm = arr.mapv(|x| x.powi(2)).sum().sqrt().max(NORMALIZE_EPS);
        arr / norm
    }

    pub fn from_unnormalized_array(embedding: ArrayD<f32>) -> Self {
        Self {
            embedding: Embedding::normalize(embedding).into_iter().collect(),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    dotenvy::dotenv().ok();
    let settings = Settings::parse();

    let rabbitmq_settings = RabbitMQSettings {
        host: std::env::var("RABBITMQ_HOST")
            .expect_or_log("RABBITMQ_HOST environment variable is not set"),
        port: std::env::var("RABBITMQ_PORT")
            .expect_or_log("RABBITMQ_PORT environment variable is not set")
            .parse()
            .expect_or_log("Can't parse RABBITMQ_PORT"),
        username: std::env::var("RABBITMQ_USERNAME")
            .expect_or_log("RABBITMQ_USERNAME environment variable is not set"),
        password: std::env::var("RABBITMQ_PASSWORD")
            .expect_or_log("RABBITMQ_PASSWORD environment variable is not set"),
    };

    let elasticsearch_url = std::env::var("ELASTICSEARCH_URL")
        .expect_or_log("ELASTICSEARCH_URL environment variable is not set");
    let elasticsearch_username = std::env::var("ELASTICSEARCH_USERNAME")
        .expect_or_log("ELASTICSEARCH_USERNAME environment variable is not set");
    let elasticsearch_password = std::env::var("ELASTICSEARCH_PASSWORD")
        .expect_or_log("ELASTICSEARCH_PASSWORD environment variable is not set");

    let es_url = Url::parse(&elasticsearch_url).unwrap_or_log();
    let es_conn_pool = SingleNodeConnectionPool::new(es_url);
    let es_transport = TransportBuilder::new(es_conn_pool)
        .auth(Credentials::Basic(
            elasticsearch_username,
            elasticsearch_password,
        ))
        .build()
        .expect_or_log("Can't create connection to Elasticsearch");
    let es_client = Elasticsearch::new(es_transport);
    create_index(&es_client)
        .await
        .expect_or_log("Can't create Elasticsearch index");
    ELASTICSEARCH.set(es_client).unwrap();

    initialize_models(&settings).expect_or_log("Can't initialize models");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let rabbitmq_task = tokio::spawn(async {
        launch_rabbitmq_connection(rabbitmq_settings, shutdown_rx).await;
    });

    shutdown_signal().await;
    shutdown_tx.send(()).unwrap();
    rabbitmq_task.await.unwrap();
}

fn initialize_models(settings: &Settings) -> anyhow::Result<()> {
    clip_image::initialize_model(settings)?;
    clip_text::initialize_model(settings)?;
    Ok(())
}

async fn launch_rabbitmq_connection(
    settings: RabbitMQSettings,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        match connect_rabbitmq(&settings, &mut shutdown_rx).await {
            Ok(_) => {
                tracing::info!("RabbitMQ connection shut down normally");
                break;
            }
            Err(err) => {
                tracing::error!("RabbitMQ connection returned error: {err:?}");
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
}

async fn connect_rabbitmq(
    settings: &RabbitMQSettings,
    shutdown_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let connection = Connection::open(&OpenConnectionArguments::new(
        &settings.host,
        settings.port,
        &settings.username,
        &settings.password,
    ))
    .await?;
    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap_or_log();

    let channel = connection.open_channel(None).await.unwrap_or_log();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap_or_log();
    *RABBITMQ_CHANNEL.write().await = Some(channel);

    let (queue_name, _, _) = RABBITMQ_CHANNEL
        .read()
        .await
        .as_ref()
        .unwrap()
        .queue_declare(
            QueueDeclareArguments::new(RABBITMQ_QUEUE_NAME)
                .durable(true)
                .finish(),
        )
        .await
        .unwrap_or_log()
        .unwrap_or_log();

    let args = BasicConsumeArguments::new(&queue_name, "");
    RABBITMQ_CHANNEL
        .read()
        .await
        .as_ref()
        .unwrap()
        .basic_consume(Consumer, args)
        .await
        .unwrap_or_log();

    tracing::info!("Listening...");

    tokio::select! {
        _ = shutdown_rx => {
            connection.close().await?;
            Ok(())
        }
        result = connection.listen_network_io_failure() => {
            if result {
                Err(anyhow::anyhow!("connection failure"))
            } else {
                Err(anyhow::anyhow!("connection shut down without IO errors"))
            }
        }
    }
}

struct Consumer;

#[async_trait]
impl AsyncConsumer for Consumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let message: WorkerMessage = serde_json::from_slice(&content).unwrap_or_log();
        let res = match message {
            WorkerMessage::OnUpload(x) => on_upload::process_request(x).await,
            WorkerMessage::Search(x) => {
                search::process_request(
                    x,
                    basic_properties.reply_to(),
                    basic_properties.correlation_id(),
                )
                .await
            }
        };
        match res {
            Ok(_) => {
                channel
                    .basic_ack(BasicAckArguments::new(deliver.delivery_tag(), false))
                    .await
            }
            Err(_) => {
                channel
                    .basic_nack(BasicNackArguments::new(deliver.delivery_tag(), false, true))
                    .await
            }
        }
        .unwrap_or_log();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect_or_log("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect_or_log("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}
