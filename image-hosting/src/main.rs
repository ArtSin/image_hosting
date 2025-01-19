#[cfg(feature = "ssr")]
use amqprs::{channel::Channel, consumer::AsyncConsumer, BasicProperties, Deliver};
#[cfg(feature = "ssr")]
use common::SearchResponse;
#[cfg(feature = "ssr")]
use image_hosting::RABBITMQ_RESPONSES;
#[cfg(feature = "ssr")]
use tokio::{signal, sync::oneshot};
#[cfg(feature = "ssr")]
use tracing_unwrap::{OptionExt, ResultExt};

#[cfg(feature = "ssr")]
struct RabbitMQSettings {
    host: String,
    port: u16,
    username: String,
    password: String,
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use common::storage::create_folders;
    use image_hosting::{app::*, components::image::get_image_file};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    dotenvy::dotenv().ok();
    let db_url =
        std::env::var("DATABASE_URL").expect_or_log("DATABASE_URL environment variable is not set");
    image_hosting::APP_SECRET
        .set(
            std::env::var("APP_SECRET").expect_or_log("APP_SECRET environment variable is not set"),
        )
        .unwrap();

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

    create_folders()
        .await
        .expect_or_log("Can't create storage folders");

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(image_hosting::MAX_DB_CONNECTIONS)
        .connect(&db_url)
        .await
        .expect_or_log("Database connection failed");
    sqlx::migrate!("../migrations")
        .run(&db)
        .await
        .expect_or_log("Database migrations failed");
    image_hosting::DB_CONN.set(db).unwrap();

    // build our application with a route
    let app = Router::new()
        .route("/api/image/:file_name", axum::routing::get(get_image_file))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            11 * 1024 * 1024,
        ));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let (shutdown_axum_tx, shutdown_axum_rx) = oneshot::channel();
    tracing::info!("Listening on http://{}", &addr);
    let axum_task = tokio::spawn(async {
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(async {
                let _ = shutdown_axum_rx.await;
            })
            .await
            .unwrap();
    });

    let (shutdown_rabbitmq_tx, shutdown_rabbitmq_rx) = oneshot::channel();
    let rabbitmq_task = tokio::spawn(async {
        launch_rabbitmq_connection(rabbitmq_settings, shutdown_rabbitmq_rx).await;
    });

    shutdown_signal().await;
    shutdown_rabbitmq_tx.send(()).unwrap();
    rabbitmq_task.await.unwrap();
    shutdown_axum_tx.send(()).unwrap();
    axum_task.await.unwrap();
}

#[cfg(feature = "ssr")]
async fn launch_rabbitmq_connection(
    settings: RabbitMQSettings,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    use std::time::Duration;

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

#[cfg(feature = "ssr")]
async fn connect_rabbitmq(
    settings: &RabbitMQSettings,
    shutdown_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    use amqprs::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        channel::{BasicConsumeArguments, QueueDeclareArguments},
        connection::{Connection, OpenConnectionArguments},
    };

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
    *image_hosting::RABBITMQ_CHANNEL.write().await = Some(channel);

    let (callback_queue_name, _, _) = image_hosting::RABBITMQ_CHANNEL
        .read()
        .await
        .as_ref()
        .unwrap()
        .queue_declare(
            QueueDeclareArguments::new(common::RABBITMQ_CALLBACK_QUEUE_NAME)
                .durable(true)
                .finish(),
        )
        .await
        .unwrap_or_log()
        .unwrap_or_log();

    let args = BasicConsumeArguments::new(&callback_queue_name, "")
        .manual_ack(true)
        .finish();
    image_hosting::RABBITMQ_CHANNEL
        .read()
        .await
        .as_ref()
        .unwrap()
        .basic_consume(Consumer, args)
        .await
        .unwrap();

    tracing::info!("Listening for RabbitMQ messages...");

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

#[cfg(feature = "ssr")]
struct Consumer;

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
impl AsyncConsumer for Consumer {
    async fn consume(
        &mut self,
        _channel: &Channel,
        _deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let message: SearchResponse = serde_json::from_slice(&content).unwrap_or_log();
        if let Some((_, sender)) =
            RABBITMQ_RESPONSES.remove(basic_properties.correlation_id().unwrap_or_log())
        {
            sender.send(message).unwrap_or_log();
        }
    }
}

#[cfg(feature = "ssr")]
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

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
