use leptos::*;

#[cfg(feature = "ssr")]
use axum::{
    http::{header::ACCEPT_LANGUAGE, HeaderMap, HeaderValue},
    response::IntoResponse,
};
#[cfg(feature = "ssr")]
use axum_extra::extract::CookieJar;
#[cfg(feature = "ssr")]
use leptos_axum::{extract, ResponseOptions};

use crate::i18n::*;

#[cfg(feature = "ssr")]
pub fn use_cookie_jar(cookie_jar: CookieJar) {
    let response = expect_context::<ResponseOptions>();
    for (key, value) in cookie_jar.into_response().into_parts().0.headers.iter() {
        response.append_header(key.clone(), value.clone());
    }
}

const SUPPORTED_LANGS: [&str; 4] = ["en-US", "en", "ru-RU", "ru"];
const SUPPORTED_LANGS_ENUM: [Locale; 4] = [Locale::en, Locale::en, Locale::ru, Locale::ru];

#[server(GetLang)]
pub async fn get_lang() -> Result<String, ServerFnError> {
    let headers: HeaderMap = extract().await.unwrap();
    let empty_val = HeaderValue::from_static("");
    let accept_langs = headers
        .get(ACCEPT_LANGUAGE)
        .unwrap_or(&empty_val)
        .to_str()
        .unwrap_or_default();
    Ok(
        accept_language::intersection(accept_langs, &SUPPORTED_LANGS)
            .into_iter()
            .next()
            .unwrap_or(SUPPORTED_LANGS[0].to_owned()),
    )
}

pub fn get_locale(lang: String) -> Locale {
    let lang_i = SUPPORTED_LANGS
        .iter()
        .position(|&x| x == lang)
        .unwrap_or_default();
    SUPPORTED_LANGS_ENUM[lang_i]
}
