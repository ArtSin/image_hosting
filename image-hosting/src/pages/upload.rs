use leptos::prelude::*;
use server_fn::codec::{MultipartData, MultipartFormData};
use web_sys::FormData;

#[cfg(feature = "ssr")]
use amqprs::{channel::BasicPublishArguments, BasicProperties};
#[cfg(feature = "ssr")]
pub use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use common::{
    storage::{get_image_format, get_image_path, store_image},
    OnUploadMessage, WorkerMessage,
};
#[cfg(feature = "ssr")]
use leptos_axum::{extract, redirect};

use crate::{
    components::status_dialog::{StatusDialog, StatusDialogState},
    i18n::*,
    image::IMAGE_ACCEPT_EXT_MIME,
    user::AuthState,
};

#[cfg(feature = "ssr")]
use crate::{
    db::image::{delete_image, insert_image},
    image::{Image, IMAGE_EXTENSIONS},
    user::decode_session_token,
    util::{get_lang, get_locale},
};

const TITLE_MIN_LEN: usize = 4;
const TITLE_MAX_LEN: usize = 256;
const IMAGE_MAX_MIB: usize = 10;
const IMAGE_MAX_BYTES: usize = IMAGE_MAX_MIB * 1024 * 1024;

#[component]
pub fn Upload() -> impl IntoView {
    let i18n = use_i18n();
    let app_state = use_context::<crate::AppState>().unwrap();

    let upload_action = Action::new_local(|data: &FormData| upload_image(data.clone().into()));
    let on_submit = move |event: web_sys::SubmitEvent| {
        use wasm_bindgen::JsCast;

        event.prevent_default();
        let target = event
            .target()
            .unwrap()
            .unchecked_into::<web_sys::HtmlFormElement>();
        let form_data = web_sys::FormData::new_with_form(&target).unwrap();

        let image = form_data.get("image").unchecked_into::<web_sys::File>();
        let image_size = image.size() as usize;
        if image_size > IMAGE_MAX_BYTES {
            app_state.status.set(StatusDialogState::Error(
                t_string!(i18n, uploading_error).to_owned() + t_string!(i18n, image_too_big),
            ));
            return;
        }

        app_state.status.set(StatusDialogState::Loading);
        upload_action.dispatch_local(form_data);
    };
    Effect::new(move |_| match upload_action.value().get() {
        Some(Ok(_)) => {
            app_state.status.set(StatusDialogState::None);
        }
        Some(Err(e)) => {
            app_state.status.set(StatusDialogState::Error(
                t_string!(i18n, uploading_error).to_owned() + &e.to_string(),
            ));
        }
        None => {}
    });

    view! {
        <StatusDialog />
        <main>
            <Show when=move || matches!(app_state.auth_state.get(), AuthState::Authorized { .. })
                fallback=move || view! { <h2>{move || { t!(i18n, not_logged_in) }}</h2> }>
                <form on:submit=on_submit>
                    <h2>{move || { t!(i18n, uploading_image) }}</h2>
                    <label for="title">
                        {move || {
                            t!(i18n, title_with_range, min = TITLE_MIN_LEN, max = TITLE_MAX_LEN)
                        }}
                    </label>
                    <input type="text" id="title" name="title" required=true
                        minlength=TITLE_MIN_LEN maxlength=TITLE_MAX_LEN size=25 />
                    <label for="image">
                        {move || {
                            t!(i18n, image_with_size, max = IMAGE_MAX_MIB)
                        }}
                    </label>
                    <input type="file" id="image" name="image" accept=IMAGE_ACCEPT_EXT_MIME required=true />
                    <button type="submit">{move || { t!(i18n, upload) }}</button>
                </form>
            </Show>
        </main>
    }
}

#[server(name = UploadImage, input = MultipartFormData)]
pub async fn upload_image(data: MultipartData) -> Result<(), ServerFnError<String>> {
    let locale = get_locale(get_lang().await.unwrap());
    let cookie_jar: CookieJar = extract().await.unwrap();
    let user = match decode_session_token(&cookie_jar) {
        AuthState::Authorized { user } => user,
        AuthState::NotAuthorized => {
            return Err(td_string!(locale, not_logged_in).to_owned().into())
        }
    };

    let mut data = data.into_inner().unwrap();
    let mut title = None;
    let mut image_bytes = None;
    while let Ok(Some(mut field)) = data.next_field().await {
        let mut buf = bytes::BytesMut::new();
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => buf.extend_from_slice(&chunk),
                Ok(None) => break,
                Err(_) => return Err(td_string!(locale, parsing_error).to_owned().into()),
            }
        }

        match field.name().unwrap_or_default() {
            "title" => {
                title = String::from_utf8(buf.to_vec()).ok();
            }
            "image" => {
                image_bytes = Some(buf.to_vec());
            }
            _ => return Err(td_string!(locale, parsing_error).to_owned().into()),
        }
    }

    if title.is_none() || image_bytes.is_none() {
        return Err(td_string!(locale, parsing_error).to_owned().into());
    }
    let title = title.unwrap();
    let image_bytes = image_bytes.unwrap();

    if title.len() < TITLE_MIN_LEN {
        return Err(td_string!(locale, title_too_short).to_owned().into());
    }
    if title.len() > TITLE_MAX_LEN {
        return Err(td_string!(locale, title_too_long).to_owned().into());
    }

    let format = get_image_format(&image_bytes, &IMAGE_EXTENSIONS)?;

    let mut image_db = Image {
        format: format.to_owned(),
        title,
        author: user.id,
        timestamp: chrono::offset::Utc::now(),
        ..Default::default()
    };
    insert_image(&mut image_db)
        .await
        .map_err(|_| td_string!(locale, db_error).to_owned())?;

    let path = get_image_path(image_db.id, format, false);
    if let e @ Err(_) = store_image(path, image_bytes).await {
        let _ = delete_image(image_db.id).await;
        e?;
    }

    let body = serde_json::to_vec(&WorkerMessage::OnUpload(OnUploadMessage {
        id: image_db.id,
        format: image_db.format,
        title: image_db.title,
    }))
    .unwrap();
    let props = BasicProperties::default().with_persistence(true).finish();
    let args = BasicPublishArguments::default()
        .routing_key(common::RABBITMQ_QUEUE_NAME.to_owned())
        .finish();
    crate::RABBITMQ_CHANNEL
        .get()
        .unwrap()
        .basic_publish(props, body, args)
        .await
        .map_err(|_| td_string!(locale, db_error).to_owned())?;

    redirect("/");
    Ok(())
}
