use leptos::prelude::*;

use crate::i18n::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusDialogState {
    None,
    Loading,
    Info(String),
    Error(String),
}

#[component]
pub fn StatusDialog() -> impl IntoView {
    let i18n = use_i18n();
    let app_state = use_context::<crate::AppState>().unwrap();

    let header_str = move || match app_state.status.get() {
        StatusDialogState::None | StatusDialogState::Loading => String::new(),
        StatusDialogState::Info(_) => t_string!(i18n, info).to_owned(),
        StatusDialogState::Error(_) => t_string!(i18n, error).to_owned(),
    };
    let message_str = move || match app_state.status.get() {
        StatusDialogState::None => String::new(),
        StatusDialogState::Loading => t_string!(i18n, loading).to_owned(),
        StatusDialogState::Info(ref x) => x.clone(),
        StatusDialogState::Error(ref x) => "❌ ".to_owned() + x,
    };

    view! {
        <Show when=move || !message_str().is_empty() fallback=|| ()>
            <div id="dialog">
                <form action="javascript:void(0);" on:submit=move |_| {
                    app_state.status.set(StatusDialogState::None);
                }>
                    <Show when=move || !header_str().is_empty() fallback=|| ()>
                        <h3>{header_str}</h3>
                    </Show>
                    <p>{message_str}</p>
                    <Show when=move || app_state.status.get() != StatusDialogState::Loading fallback=|| ()>
                        <menu>
                            <button>{move || { t!(i18n, ok) } }</button>
                        </menu>
                    </Show>
                </form>
            </div>
        </Show>
    }
}
