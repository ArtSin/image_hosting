use leptos::prelude::*;
use leptos_router::{components::Form, hooks::use_query_map};

#[cfg(feature = "ssr")]
use amqprs::{channel::BasicPublishArguments, BasicProperties};
#[cfg(feature = "ssr")]
use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use common::{SearchMessage, WorkerMessage};
#[cfg(feature = "ssr")]
use leptos_axum::extract;
#[cfg(feature = "ssr")]
use tokio::sync::oneshot;

use crate::{
    components::images::Images, i18n::*, image::Image, image_votes::ImageVotes, user::User,
};

#[cfg(feature = "ssr")]
use crate::{
    db::image::get_images_with_authors_and_votes_by_ids,
    user::{decode_session_token, AuthState},
    util::{get_lang, get_locale},
};

#[component]
pub fn Search() -> impl IntoView {
    let i18n = use_i18n();
    let query = use_query_map();
    let query_text_and_page = move || {
        (
            query.get().get("query_text"),
            query
                .get()
                .get("page")
                .map(|x| x.parse())
                .transpose()
                .ok()
                .flatten(),
        )
    };
    let images = Resource::new_blocking(query_text_and_page, move |x| async move {
        search_images(x.0, x.1).await
    });
    let query_str = move || {
        let (query_text, page) = query_text_and_page();
        format!(
            "?{}page={}",
            query_text
                .map(|x| format!("query_text={x}&"))
                .unwrap_or_default(),
            page.unwrap_or_default() + 1
        )
    };

    view! {
        <header>
            <Form action="" method="get" class:search=true>
                <input type="search" id="query_text" name="query_text" required=true />
                <button type="submit">{move || { t!(i18n, search) }}</button>
            </Form>
        </header>
        <Images images=images query_str=query_str />
    }
}

#[server(GetAllImages)]
pub async fn search_images(
    query_text: Option<String>,
    page: Option<i64>,
) -> Result<(Vec<(Image, User, ImageVotes)>, bool), ServerFnError<String>> {
    if query_text.is_none() {
        return Ok((Vec::new(), true));
    }

    let locale = get_locale(get_lang().await.unwrap());
    let cookie_jar: CookieJar = extract().await.unwrap();
    let curr_user_id = match decode_session_token(&cookie_jar) {
        AuthState::Authorized { user } => user.id,
        AuthState::NotAuthorized => -1,
    };

    let body = serde_json::to_vec(&WorkerMessage::Search(SearchMessage {
        query_text: query_text.unwrap(),
        page: page.unwrap_or_default(),
    }))
    .unwrap();

    let (tx, rx) = oneshot::channel();
    let correlation_id = uuid::Uuid::new_v4().to_string();
    let props = BasicProperties::default()
        .with_persistence(true)
        .with_reply_to(crate::RABBITMQ_CALLBACK_QUEUE.get().unwrap())
        .with_correlation_id(&correlation_id)
        .finish();
    let args = BasicPublishArguments::default()
        .routing_key(common::RABBITMQ_QUEUE_NAME.to_owned())
        .finish();
    crate::RABBITMQ_RESPONSES.insert(correlation_id, tx);
    crate::RABBITMQ_CHANNEL
        .get()
        .unwrap()
        .basic_publish(props, body, args)
        .await
        .map_err(|_| td_string!(locale, db_error).to_owned())?;

    let response = rx
        .await
        .map_err(|_| td_string!(locale, db_error).to_owned())?;

    get_images_with_authors_and_votes_by_ids(curr_user_id, response.ids)
        .await
        .map(|x| (x, response.last_page))
        .map_err(|_| td_string!(locale, db_error).to_owned().into())
}
