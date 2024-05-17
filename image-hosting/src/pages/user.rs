use chrono::{DateTime, Utc};
use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use leptos_axum::extract;

use crate::{
    components::images::{Images, IMAGES_PER_PAGE},
    i18n::*,
    image::Image,
    image_votes::ImageVotes,
    user::User,
};

#[cfg(feature = "ssr")]
use crate::{
    db::image::get_all_images_with_authors_and_votes_by_author,
    user::{decode_session_token, AuthState},
    util::{get_lang, get_locale},
};

#[derive(Debug, Clone, Params, PartialEq, Eq)]
struct UserParams {
    id: Option<i64>,
}

#[component]
pub fn User() -> impl IntoView {
    let params = use_params::<UserParams>();
    let query = use_query_map();
    let id_and_last_timestamp = move || {
        (
            params.get().map(|x| x.id).ok().flatten().unwrap_or(-1),
            query
                .get()
                .get("last")
                .map(|x| x.parse())
                .transpose()
                .ok()
                .flatten()
                .map(|x| DateTime::<Utc>::from_timestamp_micros(x))
                .flatten(),
        )
    };
    let images = create_blocking_resource(id_and_last_timestamp, move |(id, t)| async move {
        get_all_images_by_author(id, t).await
    });

    view! {
        <Images images=images />
    }
}

#[server(GetAllImages)]
pub async fn get_all_images_by_author(
    author_id: i64,
    last_timestamp: Option<DateTime<Utc>>,
) -> Result<(Vec<(Image, User, ImageVotes)>, bool), ServerFnError<String>> {
    let locale = get_locale(get_lang().await.unwrap());
    let cookie_jar: CookieJar = extract().await.unwrap();
    let curr_user_id = match decode_session_token(&cookie_jar) {
        AuthState::Authorized { user } => user.id,
        AuthState::NotAuthorized => -1,
    };
    get_all_images_with_authors_and_votes_by_author(
        curr_user_id,
        IMAGES_PER_PAGE,
        author_id,
        last_timestamp,
    )
    .await
    .map_err(|_| td!(locale, db_error)().to_owned().into())
}
