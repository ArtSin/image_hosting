use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use leptos_axum::extract;

use crate::{
    components::image::ImageComp, i18n::*, image::Image, image_votes::ImageVotes, user::User,
};

#[cfg(feature = "ssr")]
use crate::{
    db::image::get_image_with_authors_and_votes_by_id,
    user::{decode_session_token, AuthState},
    util::{get_lang, get_locale},
};

#[derive(Debug, Clone, Params, PartialEq, Eq)]
struct ImageParams {
    id: Option<i64>,
}

#[component]
pub fn Image() -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params::<ImageParams>();
    let id = move || params.get().map(|x| x.id).ok().flatten();

    let image = create_blocking_resource(
        id,
        move |id| async move { get_image(id.unwrap_or(-1)).await },
    );

    let show_error = move || match image.get() {
        Some(Err(e)) => view! {
            <main>
                <h2>{move || { t!(i18n, connection_error)().to_owned() + &e.to_string() }}</h2>
            </main>
        }
        .into_view(),
        _ => view! {}.into_view(),
    };

    view! {
        <Suspense fallback=|| view! {}>
            <Show when=move || matches!(image.get(), Some(Ok(_))) fallback=show_error>
                <main>
                    {move || {
                        let x = image.get().unwrap().unwrap();
                        view! {
                            <ImageComp image={x.0} author={x.1} image_votes={x.2} />
                        }
                    }}
                </main>
            </Show>
        </Suspense>
    }
}

#[server(GetImage)]
pub async fn get_image(id: i64) -> Result<(Image, User, ImageVotes), ServerFnError<String>> {
    let locale = get_locale(get_lang().await.unwrap());
    let cookie_jar: CookieJar = extract().await.unwrap();
    let curr_user_id = match decode_session_token(&cookie_jar) {
        AuthState::Authorized { user } => user.id,
        AuthState::NotAuthorized => -1,
    };
    get_image_with_authors_and_votes_by_id(id, curr_user_id)
        .await
        .map_err(|_| td!(locale, db_error)().to_owned())?
        .ok_or_else(|| td!(locale, nothing_found)().to_owned().into())
}
