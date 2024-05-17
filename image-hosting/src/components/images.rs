use leptos::*;
use leptos_router::*;

use crate::{
    components::image::ImageComp, i18n::*, image::Image, image_votes::ImageVotes, user::User,
};

pub const IMAGES_PER_PAGE: i64 = 6;

pub type ImagesData = (Vec<(Image, User, ImageVotes)>, bool);

#[component]
pub fn Images<T>(images: Resource<T, Result<ImagesData, ServerFnError<String>>>) -> impl IntoView
where
    T: 'static + Clone,
{
    let i18n = use_i18n();

    let show_error = move || match images.get() {
        Some(Err(e)) => view! {
            <main>
                <h2>{move || { t!(i18n, connection_error)().to_owned() + &e.to_string() }}</h2>
            </main>
        }
        .into_view(),
        _ => view! {}.into_view(),
    };

    view! {
        <Suspense fallback=move || view! {}>
            <Show when=move || matches!(images.get(), Some(Ok(_))) fallback=show_error>
                <main>
                    {move || {
                        let images = move || images.get().unwrap().unwrap();
                        if images().0.is_empty() {
                            view! { <h2>{move || { t!(i18n, nothing_found) }}</h2> }.into_view()
                        } else {
                            view! {
                                <For each=move || images().0 key=|x| x.0.id children=move |x| {
                                    view! {
                                        <ImageComp image={x.0} author={x.1} image_votes={x.2} />
                                    }
                                } />
                                {move || {
                                    if images().1 {
                                        view! {}.into_view()
                                    } else {
                                        let link = format!("?last={}", images().0.last().unwrap().0.timestamp.timestamp_micros());
                                        view! {
                                            <A class="next_page" href={link}><button>"⟶"</button></A>
                                        }.into_view()
                                    }
                                }}
                            }.into_view()
                        }
                    }}
                </main>
            </Show>
        </Suspense>
    }
}
