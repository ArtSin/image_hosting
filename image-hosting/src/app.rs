use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path, SsrMode};

use crate::{
    components::nav_tabs::NavTabs,
    error_template::{AppError, ErrorTemplate},
    i18n::*,
    pages::{
        image::Image, index::Index, login::LogIn, logout::LogOut, register::Register,
        search::Search, upload::Upload, user::User,
    },
    user::{self, get_auth_state},
    util::{get_lang, get_locale},
    AppState, StatusDialogState,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let default_lang = Resource::new_blocking(|| (), |_| async { get_lang().await });
    let auth_state = Resource::new_blocking(|| (), |_| async { get_auth_state().await });
    provide_context(AppState {
        auth_state: RwSignal::new(user::AuthState::NotAuthorized),
        status: RwSignal::new(StatusDialogState::None),
    });
    let context = use_context::<crate::AppState>().unwrap();

    view! {
        <Stylesheet href="/water.min.css"/>

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/image-hosting.css"/>

        // sets the document title
        <Title text="Image hosting"/>

        <I18nContextProvider>
        <Router>
            <Suspense fallback=|| ()>
                {move || {
                    let lang = default_lang.get().unwrap_or(Ok(String::new())).unwrap_or_default();
                    use_i18n().set_locale(get_locale(lang));

                    context.auth_state.set(auth_state
                        .get()
                        .unwrap_or(Ok(user::AuthState::NotAuthorized))
                        .unwrap_or(user::AuthState::NotAuthorized));
                }}

                <NavTabs />
                <Routes fallback=|| {
                    let mut outside_errors = Errors::default();
                    outside_errors.insert_with_default_key(AppError::NotFound);
                    view! {
                        <ErrorTemplate outside_errors/>
                    }
                    .into_view()
                }>
                    <Route path=path!("") view=Index ssr=SsrMode::Async />
                    <Route path=path!("search") view=Search ssr=SsrMode::Async />
                    <Route path=path!("upload") view=Upload ssr=SsrMode::Async />
                    <Route path=path!("login") view=LogIn ssr=SsrMode::Async />
                    <Route path=path!("register") view=Register ssr=SsrMode::Async />
                    <Route path=path!("user/:id") view=User ssr=SsrMode::Async />
                    <Route path=path!("logout") view=LogOut ssr=SsrMode::Async />
                    <Route path=path!("image/:id") view=Image ssr=SsrMode::Async />
                </Routes>
            </Suspense>
        </Router>
        </I18nContextProvider>
    }
}
