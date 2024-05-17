#[cfg(feature = "ssr")]
use std::sync::OnceLock;

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
pub mod storage;
pub mod user;
pub mod util;

leptos_i18n::load_locales!();

#[cfg(feature = "ssr")]
pub const MAX_DB_CONNECTIONS: u32 = 5;
#[cfg(feature = "ssr")]
pub static DB_CONN: OnceLock<sqlx::PgPool> = OnceLock::new();
#[cfg(feature = "ssr")]
pub static APP_SECRET: OnceLock<String> = OnceLock::new();

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
