use leptos::server;
use serde::{Deserialize, Serialize};
use server_fn::ServerFnError;

#[cfg(feature = "ssr")]
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
#[cfg(feature = "ssr")]
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
#[cfg(feature = "ssr")]
use leptos_axum::extract;

#[cfg(feature = "ssr")]
const SESSION_COOKIE: &str = "session_token";
#[cfg(feature = "ssr")]
const SESSION_DURATION: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: -1,
            name: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthState {
    NotAuthorized,
    Authorized { user: User },
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

#[cfg(feature = "ssr")]
pub fn create_session_token(user: &User, cookie_jar: CookieJar) -> CookieJar {
    let exp_seconds = jsonwebtoken::get_current_timestamp() + SESSION_DURATION;
    let claims = Claims {
        sub: serde_json::to_string(user).unwrap(),
        exp: exp_seconds,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(crate::APP_SECRET.get().unwrap().as_bytes()),
    )
    .unwrap();

    cookie_jar.add(
        Cookie::build((SESSION_COOKIE, token))
            .expires(time::OffsetDateTime::from_unix_timestamp(exp_seconds as i64).unwrap())
            .http_only(true)
            .same_site(SameSite::Strict)
            .path("/"),
    )
}

#[cfg(feature = "ssr")]
pub fn remove_session_token(cookie_jar: CookieJar) -> CookieJar {
    cookie_jar.remove(Cookie::build((SESSION_COOKIE, "")).path("/"))
}

/// Decode and validate session token
#[cfg(feature = "ssr")]
pub fn decode_session_token(cookie_jar: &CookieJar) -> AuthState {
    let session_token = cookie_jar.get(SESSION_COOKIE).and_then(|cookie| {
        decode::<Claims>(
            cookie.value(),
            &DecodingKey::from_secret(crate::APP_SECRET.get().unwrap().as_bytes()),
            &Validation::default(),
        )
        .ok()
    });
    let user = session_token.and_then(|token| serde_json::from_str(&token.claims.sub).ok());
    match user {
        Some(user) => AuthState::Authorized { user },
        None => AuthState::NotAuthorized,
    }
}

#[server(GetAuthState)]
pub async fn get_auth_state() -> Result<AuthState, ServerFnError> {
    let cookie_jar: CookieJar = extract().await.unwrap();
    Ok(decode_session_token(&cookie_jar))
}
