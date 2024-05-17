use leptos::*;
use leptos_router::ActionForm;

#[cfg(feature = "ssr")]
pub use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use leptos_axum::{extract, redirect};
#[cfg(feature = "ssr")]
use libreauth::pass::HashBuilder;

use crate::{
    components::status_dialog::{StatusDialog, StatusDialogState},
    i18n::*,
    pages::register::{PASSWORD_MAX_LEN, PASSWORD_MIN_LEN, USER_NAME_MAX_LEN, USER_NAME_MIN_LEN},
    user::{AuthState, User},
    util::{get_lang, get_locale},
};

#[cfg(feature = "ssr")]
use crate::{
    db::user::get_user_with_password_hash_by_name,
    user::{create_session_token, decode_session_token},
    util::use_cookie_jar,
};

#[component]
pub fn LogIn() -> impl IntoView {
    let i18n = use_i18n();
    let app_state = use_context::<crate::AppState>().unwrap();

    let login_action = create_server_action::<LogIn>();
    let on_submit = move |_| {
        app_state.status.set(StatusDialogState::Loading);
    };
    create_effect(move |_| match login_action.value().get() {
        Some(Ok(user)) => {
            app_state.status.set(StatusDialogState::None);
            app_state.auth_state.set(AuthState::Authorized { user });
        }
        Some(Err(e)) => {
            app_state.status.set(StatusDialogState::Error(
                t!(i18n, login_error)().to_owned() + &e.to_string(),
            ));
        }
        None => {}
    });

    view! {
        <StatusDialog />
        <main>
            <Show when=move || matches!(app_state.auth_state.get(), AuthState::NotAuthorized)
                fallback=move || view! { <h2>{move || { t!(i18n, already_logged_in) }}</h2> }>
                <ActionForm action=login_action on:submit=on_submit>
                    <h2>{move || { t!(i18n, logging_in) }}</h2>
                    <label for="user_name">{move || { t!(i18n, user_name) }}</label>
                    <input type="text" id="user_name" name="user_name" required=true
                        minlength=USER_NAME_MIN_LEN maxlength=USER_NAME_MAX_LEN size=25 />
                    <label for="password">{move || { t!(i18n, password) }}</label>
                    <input type="password" id="password" name="password" required=true
                        minlength=PASSWORD_MIN_LEN maxlength=PASSWORD_MAX_LEN size=25 />
                    <button type="submit">{move || { t!(i18n, log_in) }}</button>
                </ActionForm>
            </Show>
        </main>
    }
}

#[server(name = LogIn)]
pub async fn login_user(
    user_name: String,
    password: String,
) -> Result<User, ServerFnError<String>> {
    let locale = get_locale(get_lang().await.unwrap());
    let cookie_jar: CookieJar = extract().await.unwrap();
    if let AuthState::Authorized { .. } = decode_session_token(&cookie_jar) {
        return Err(td!(locale, already_logged_in)().to_owned().into());
    }

    if user_name.len() < USER_NAME_MIN_LEN {
        return Err(td!(locale, user_name_too_short)().to_owned().into());
    }
    if user_name.len() > USER_NAME_MAX_LEN {
        return Err(td!(locale, user_name_too_long)().to_owned().into());
    }
    if password.len() < PASSWORD_MIN_LEN {
        return Err(td!(locale, password_too_short)().to_owned().into());
    }
    if password.len() > PASSWORD_MAX_LEN {
        return Err(td!(locale, password_too_long)().to_owned().into());
    }

    let (user, password_hash) = get_user_with_password_hash_by_name(&user_name)
        .await
        .map_err(|_| td!(locale, db_error)().to_owned())?
        .ok_or(td!(locale, user_name_incorrect)().to_owned())?;

    let checker = HashBuilder::from_phc(&password_hash).unwrap();
    if !checker.is_valid(&password) {
        return Err(td!(locale, password_incorrect)().to_owned().into());
    }

    use_cookie_jar(create_session_token(&user, cookie_jar));
    redirect("/");
    Ok(user)
}
