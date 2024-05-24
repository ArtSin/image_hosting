#[cfg(feature = "ssr")]
use std::sync::OnceLock;

#[cfg(feature = "ssr")]
use common::SearchResponse;
#[cfg(feature = "ssr")]
use dashmap::DashMap;
#[cfg(feature = "ssr")]
use once_cell::sync::Lazy;
#[cfg(feature = "ssr")]
use tokio::sync::oneshot;

use components::status_dialog::StatusDialogState;
use leptos::*;

pub mod app;
pub mod components;
pub mod db;
pub mod error_template;
pub mod fileserv;
pub mod image;
pub mod image_votes;
pub mod pages;
pub mod user;
pub mod util;

leptos_i18n::load_locales!();

#[cfg(feature = "ssr")]
pub const MAX_DB_CONNECTIONS: u32 = 5;
#[cfg(feature = "ssr")]
pub static DB_CONN: OnceLock<sqlx::PgPool> = OnceLock::new();
#[cfg(feature = "ssr")]
pub static APP_SECRET: OnceLock<String> = OnceLock::new();
#[cfg(feature = "ssr")]
pub static RABBITMQ_CHANNEL: OnceLock<amqprs::channel::Channel> = OnceLock::new();
#[cfg(feature = "ssr")]
pub static RABBITMQ_CALLBACK_QUEUE: OnceLock<String> = OnceLock::new();
#[cfg(feature = "ssr")]
pub static RABBITMQ_RESPONSES: Lazy<DashMap<String, oneshot::Sender<SearchResponse>>> =
    Lazy::new(|| DashMap::new());

#[derive(Debug, Clone)]
struct AppState {
    auth_state: RwSignal<user::AuthState>,
    status: RwSignal<StatusDialogState>,
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
