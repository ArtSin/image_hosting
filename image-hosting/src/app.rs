use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::{
    components::nav_tabs::NavTabs,
    error_template::{AppError, ErrorTemplate},
    i18n::*,
    pages::{
        image::Image, index::Index, login::LogIn, logout::LogOut, register::Register,
        upload::Upload, user::User,
    },
    user::{self, get_auth_state},
    util::{get_lang, get_locale},
    AppState, StatusDialogState,
};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_i18n_context();

    let default_lang = create_blocking_resource(|| (), |_| async { get_lang().await });
    let auth_state = create_blocking_resource(|| (), |_| async { get_auth_state().await });
    provide_context(AppState {
        auth_state: create_rw_signal(user::AuthState::NotAuthorized),
        status: create_rw_signal(StatusDialogState::None),
    });
    let context = use_context::<crate::AppState>().unwrap();

    view! {
        <Stylesheet href="/water.min.css"/>

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/image-hosting.css"/>

        // sets the document title
        <Title text="Image hosting"/>

        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <Suspense fallback=|| view! {}>
                {move || {
                    let lang = default_lang.get().unwrap_or(Ok(String::new())).unwrap_or_default();
                    use_i18n().set_locale(get_locale(lang));

                    context.auth_state.set(auth_state
                        .get()
                        .unwrap_or(Ok(user::AuthState::NotAuthorized))
                        .unwrap_or(user::AuthState::NotAuthorized));
                }}

                <NavTabs />
                <Routes>
                    <Route path="" view=Index ssr=SsrMode::Async />
                    <Route path="login" view=LogIn ssr=SsrMode::Async />
                    <Route path="register" view=Register ssr=SsrMode::Async />
                    <Route path="user/:id" view=User ssr=SsrMode::Async />
                    <Route path="logout" view=LogOut ssr=SsrMode::Async />
                    <Route path="upload" view=Upload ssr=SsrMode::Async />
                    <Route path="image/:id" view=Image ssr=SsrMode::Async />
                </Routes>
            </Suspense>
        </Router>
    }
}
