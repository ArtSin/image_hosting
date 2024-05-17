use chrono::{DateTime, Utc};
use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use leptos_axum::extract;

use crate::{components::images::Images, image::Image, image_votes::ImageVotes, user::User};

#[cfg(feature = "ssr")]
use crate::{
    components::images::IMAGES_PER_PAGE,
    db::image::get_all_images_with_authors_and_votes,
    i18n::*,
    user::{decode_session_token, AuthState},
    util::{get_lang, get_locale},
};

#[component]
pub fn Index() -> impl IntoView {
    let query = use_query_map();
    let last_timestamp = move || {
        query
            .get()
            .get("last")
            .map(|x| x.parse())
            .transpose()
            .ok()
            .flatten()
            .and_then(DateTime::<Utc>::from_timestamp_micros)
    };
    let images =
        create_blocking_resource(
            last_timestamp,
            move |t| async move { get_all_images(t).await },
        );

    view! {
        <Images images=images />
    }
}

#[server(GetAllImages)]
pub async fn get_all_images(
    last_timestamp: Option<DateTime<Utc>>,
) -> Result<(Vec<(Image, User, ImageVotes)>, bool), ServerFnError<String>> {
    let locale = get_locale(get_lang().await.unwrap());
    let cookie_jar: CookieJar = extract().await.unwrap();
    let curr_user_id = match decode_session_token(&cookie_jar) {
        AuthState::Authorized { user } => user.id,
        AuthState::NotAuthorized => -1,
    };
    get_all_images_with_authors_and_votes(curr_user_id, IMAGES_PER_PAGE, last_timestamp)
        .await
        .map_err(|_| td!(locale, db_error)().to_owned().into())
}
