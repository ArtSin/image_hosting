use std::hash::{DefaultHasher, Hash, Hasher};

use leptos::*;
use leptos_i18n::I18nContext;
use leptos_router::*;

use crate::{
    i18n::*,
    user::{AuthState, User},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NavTabsPages {
    Index,
    LogIn,
    Register,
    User(User),
    LogOut,
    Upload,
}

impl NavTabsPages {
    fn id(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }

    fn text(self, i18n: I18nContext<Locale>) -> Box<dyn Fn() -> String> {
        match self {
            Self::Index => Box::new(move || t!(i18n, index)().to_owned()),
            Self::LogIn => Box::new(move || t!(i18n, log_in)().to_owned()),
            Self::Register => Box::new(move || t!(i18n, register)().to_owned()),
            Self::User(user) => Box::new(move || user.name.clone()),
            Self::LogOut => Box::new(move || t!(i18n, log_out)().to_owned()),
            Self::Upload => Box::new(move || t!(i18n, upload)().to_owned()),
        }
    }

    fn link(&self) -> String {
        match self {
            Self::Index => "/".to_owned(),
            Self::LogIn => "/login".to_owned(),
            Self::Register => "/register".to_owned(),
            Self::User(user) => format!("/user/{}", user.id),
            Self::LogOut => "/logout".to_owned(),
            Self::Upload => "/upload".to_owned(),
        }
    }

    fn li_class(&self) -> &'static str {
        match self {
            Self::LogIn | Self::User(_) => "end",
            _ => "",
        }
    }
}

#[component]
pub fn NavTabs() -> impl IntoView {
    let i18n = use_i18n();
    let app_state = use_context::<crate::AppState>().unwrap();
    let tabs = move || {
        let mut tabs = vec![NavTabsPages::Index];
        match app_state.auth_state.get() {
            AuthState::NotAuthorized => {
                tabs.push(NavTabsPages::LogIn);
                tabs.push(NavTabsPages::Register);
            }
            AuthState::Authorized { user } => {
                tabs.push(NavTabsPages::Upload);
                tabs.push(NavTabsPages::User(user));
                tabs.push(NavTabsPages::LogOut);
            }
        }
        tabs
    };

    view! {
        <nav>
            <ul>
                <For each=tabs key=|x| x.id() children=move |tab| {
                    view! {
                        <li class={tab.li_class()}>
                            <A href={tab.link()}>{tab.text(i18n)}</A>
                        </li>
                    }
                } />
            </ul>
        </nav>
    }
}
