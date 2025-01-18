use leptos::prelude::*;

#[cfg(feature = "ssr")]
pub use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use leptos_axum::{extract, redirect};

use crate::{
    components::status_dialog::{StatusDialog, StatusDialogState},
    i18n::*,
    user::AuthState,
};

#[cfg(feature = "ssr")]
use crate::{user::remove_session_token, util::use_cookie_jar};

#[component]
pub fn LogOut() -> impl IntoView {
    let i18n = use_i18n();
    let app_state = use_context::<crate::AppState>().unwrap();

    let logout_action = ServerAction::<LogOut>::new();
    let on_submit = move |_| {
        app_state.status.set(StatusDialogState::Loading);
    };
    Effect::new(move |_| match logout_action.value().get() {
        Some(Ok(_)) => {
            app_state.status.set(StatusDialogState::None);
            app_state.auth_state.set(AuthState::NotAuthorized);
        }
        Some(Err(e)) => {
            app_state.status.set(StatusDialogState::Error(
                t_string!(i18n, logout_error).to_owned() + &e.to_string(),
            ));
        }
        None => {}
    });

    view! {
        <StatusDialog />
        <main>
            <Show when=move || matches!(app_state.auth_state.get(), AuthState::Authorized { .. })
                fallback=move || view! { <h2>{move || { t!(i18n, not_logged_in) }}</h2> }>
                <ActionForm action=logout_action on:submit=on_submit>
                    <h2>{move || { t!(i18n, logging_out) }}</h2>
                    <button type="submit">{move || { t!(i18n, log_out) }}</button>
                </ActionForm>
            </Show>
        </main>
    }
}

#[server(name = LogOut)]
pub async fn logout_user() -> Result<(), ServerFnError<String>> {
    let cookie_jar: CookieJar = extract().await.unwrap();
    use_cookie_jar(remove_session_token(cookie_jar));
    redirect("/");
    Ok(())
}
