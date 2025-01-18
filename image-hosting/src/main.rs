#[cfg(feature = "ssr")]
use amqprs::{channel::Channel, consumer::AsyncConsumer, BasicProperties, Deliver};
#[cfg(feature = "ssr")]
use common::SearchResponse;
#[cfg(feature = "ssr")]
use image_hosting::RABBITMQ_RESPONSES;
#[cfg(feature = "ssr")]
use tracing_unwrap::{OptionExt, ResultExt};

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use amqprs::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        channel::{BasicConsumeArguments, QueueDeclareArguments},
        connection::{Connection, OpenConnectionArguments},
    };
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
    let rabbitmq_host = std::env::var("RABBITMQ_HOST")
        .expect_or_log("RABBITMQ_HOST environment variable is not set");
    let rabbitmq_port = std::env::var("RABBITMQ_PORT")
        .expect_or_log("RABBITMQ_PORT environment variable is not set")
        .parse()
        .expect_or_log("Can't parse RABBITMQ_PORT");
    let rabbitmq_username = std::env::var("RABBITMQ_USERNAME")
        .expect_or_log("RABBITMQ_USERNAME environment variable is not set");
    let rabbitmq_password = std::env::var("RABBITMQ_PASSWORD")
        .expect_or_log("RABBITMQ_PASSWORD environment variable is not set");

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

    let connection = Connection::open(&OpenConnectionArguments::new(
        &rabbitmq_host,
        rabbitmq_port,
        &rabbitmq_username,
        &rabbitmq_password,
    ))
    .await
    .expect_or_log("Can't connect to RabbitMQ");
    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap_or_log();

    let channel = connection.open_channel(None).await.unwrap_or_log();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap_or_log();
    image_hosting::RABBITMQ_CHANNEL
        .set(channel)
        .map_err(|_| ())
        .unwrap();

    let (callback_queue_name, _, _) = image_hosting::RABBITMQ_CHANNEL
        .get()
        .unwrap()
        .queue_declare(
            QueueDeclareArguments::new("")
                .exclusive(true)
                .durable(true)
                .finish(),
        )
        .await
        .unwrap_or_log()
        .unwrap_or_log();
    image_hosting::RABBITMQ_CALLBACK_QUEUE
        .set(callback_queue_name.clone())
        .unwrap();

    tokio::spawn(async move {
        let mut args = BasicConsumeArguments::new(&callback_queue_name, "");
        args.no_ack = true;
        image_hosting::RABBITMQ_CHANNEL
            .get()
            .unwrap()
            .basic_consume(Consumer, args)
            .await
            .unwrap();
    });

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
    leptos::logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
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

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
