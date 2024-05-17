use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
use axum::{extract::Path, http::header, http::StatusCode, response::IntoResponse};
#[cfg(feature = "ssr")]
use chrono::{DateTime, Utc};

use crate::{image::Image, image_votes::ImageVotes, user::User};

#[cfg(feature = "ssr")]
use crate::{
    image::{IMAGE_EXTENSIONS, IMAGE_MIME},
    storage::{get_image_metadata, get_image_path, load_image},
};

#[component]
pub fn ImageComp(image: Image, author: User, image_votes: ImageVotes) -> impl IntoView {
    let img_path = format!("/api/image/{}.{}", image.id, image.format);
    let (rating, set_rating) = create_signal(image_votes.rating);
    let (upvoted, set_upvoted) = create_signal(image_votes.curr_user_upvote == Some(true));
    let (downvoted, set_downvoted) = create_signal(image_votes.curr_user_upvote == Some(true));

    view! {
        <article class="image">
            <h3>
                <A href={format!("/image/{}", image.id)}>{image.title}</A>
            </h3>
            <img src=img_path />
            <div>
                <p>
                    <button
                        on:click=move |_| {
                            set_upvoted.set(!upvoted.get());
                            // TODO
                        }
                        class={move || if upvoted.get() { "voted" } else { "" }}>
                        "▲"
                    </button>
                    {move || rating.get().to_string()}
                    <button on:click=move |_| {
                            set_downvoted.set(!downvoted.get());
                        }
                        class={move || if downvoted.get() { "voted" } else { "" }}>
                        "▼"
                    </button>
                </p>
                <h4>
                    <A href={format!("/user/{}", author.id)}>{author.name}</A>
                </h4>
            </div>
        </article>
    }
}

#[cfg(feature = "ssr")]
pub async fn get_image_file(
    Path(file_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let dot_pos = file_name
        .find('.')
        .ok_or_else(|| (StatusCode::BAD_REQUEST, String::new()))?;
    let id = file_name[..dot_pos]
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, String::new()))?;
    let format = &file_name[(dot_pos + 1)..];
    let format_ind = match IMAGE_EXTENSIONS.iter().position(|&x| x == format) {
        Some(x) => x,
        None => return Err((StatusCode::BAD_REQUEST, String::new())),
    };

    let path = get_image_path(id, format);
    let modified = get_image_metadata(&path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .modified()
        .unwrap();
    let last_modified = DateTime::<Utc>::from(modified).to_rfc2822();

    match load_image(path).await {
        Ok(x) => Ok((
            [
                (header::CONTENT_TYPE, IMAGE_MIME[format_ind].to_owned()),
                (header::LAST_MODIFIED, last_modified),
                (header::CACHE_CONTROL, "max-age=31536000".to_owned()),
            ],
            x,
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
