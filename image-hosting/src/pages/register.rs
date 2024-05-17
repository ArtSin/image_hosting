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
    user::{AuthState, User},
    util::{get_lang, get_locale},
};

#[cfg(feature = "ssr")]
use crate::{
    db::user::{get_user_id_by_name, insert_user},
    user::{create_session_token, decode_session_token},
    util::use_cookie_jar,
};

pub(super) const USER_NAME_MIN_LEN: usize = 4;
pub(super) const USER_NAME_MAX_LEN: usize = 32;
pub(super) const PASSWORD_MIN_LEN: usize = 8;
pub(super) const PASSWORD_MAX_LEN: usize = 128;

#[component]
pub fn Register() -> impl IntoView {
    let i18n = use_i18n();
    let app_state = use_context::<crate::AppState>().unwrap();

    let register_action = create_server_action::<Register>();
    let on_submit = move |_| {
        app_state.status.set(StatusDialogState::Loading);
    };
    create_effect(move |_| match register_action.value().get() {
        Some(Ok(user)) => {
            app_state.status.set(StatusDialogState::None);
            app_state.auth_state.set(AuthState::Authorized { user });
        }
        Some(Err(e)) => {
            app_state.status.set(StatusDialogState::Error(
                t!(i18n, registration_error)().to_owned() + &e.to_string(),
            ));
        }
        None => {}
    });

    view! {
        <StatusDialog />
        <main>
            <Show when=move || matches!(app_state.auth_state.get(), AuthState::NotAuthorized)
                fallback=move || view! { <h2>{move || { t!(i18n, already_logged_in) }}</h2> }>
                <ActionForm action=register_action on:submit=on_submit>
                    <h2>{move || { t!(i18n, registration) }}</h2>
                    <label for="user_name">
                        {move || {
                            t!(i18n, user_name_with_range, min = USER_NAME_MIN_LEN, max = USER_NAME_MAX_LEN)
                        }}
                    </label>
                    <input type="text" id="user_name" name="user_name" required=true
                        minlength=USER_NAME_MIN_LEN maxlength=USER_NAME_MAX_LEN size=25 />
                    <label for="password">
                        {move || {
                            t!(i18n, password_with_range, min = PASSWORD_MIN_LEN, max = PASSWORD_MAX_LEN)
                        }}
                    </label>
                    <input type="password" id="password" name="password" required=true
                        minlength=PASSWORD_MIN_LEN maxlength=PASSWORD_MAX_LEN size=25 />
                    <button type="submit">{move || { t!(i18n, register) }}</button>
                </ActionForm>
            </Show>
        </main>
    }
}

#[server(name = Register)]
pub async fn register_user(
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

    if get_user_id_by_name(&user_name)
        .await
        .map_err(|_| td!(locale, db_error)().to_owned())?
        .is_some()
    {
        return Err(td!(locale, user_already_exists)().to_owned().into());
    }

    let hasher = HashBuilder::new()
        .version(1)
        .min_len(PASSWORD_MIN_LEN)
        .max_len(PASSWORD_MAX_LEN)
        .finalize()
        .unwrap();
    let password_hash = hasher.hash(&password).unwrap();

    let mut user = User {
        id: 0,
        name: user_name,
    };
    insert_user(&mut user, &password_hash)
        .await
        .map_err(|_| td!(locale, db_error)().to_owned())?;

    use_cookie_jar(create_session_token(&user, cookie_jar));
    redirect("/");
    Ok(user)
}
