#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use image_hosting::fileserv::file_and_error_handler;
    use image_hosting::{app::*, components::image::get_image_file};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    dotenvy::dotenv().ok();
    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable is not set");
    image_hosting::APP_SECRET
        .set(std::env::var("APP_SECRET").expect("APP_SECRET environment variable is not set"))
        .expect("Can't set app secret");

    image_hosting::DB_CONN
        .set(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(image_hosting::MAX_DB_CONNECTIONS)
                .connect(&db_url)
                .await
                .expect("Database connection failed"),
        )
        .expect("Can't set database connection");

    // build our application with a route
    let app = Router::new()
        .route("/api/image/:file_name", axum::routing::get(get_image_file))
        .leptos_routes(&leptos_options, routes, App)
        .fallback(file_and_error_handler)
        .with_state(leptos_options)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            11 * 1024 * 1024,
        ));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
