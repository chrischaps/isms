//! Authentication (TDD 10.2, 14): magic links, server sessions, API keys.
//!
//! Every secret is a random 256-bit token; the store keeps only its SHA-256.
//! (TDD 14 names argon2 for secrets at rest; a slow hash exists to protect
//! low-entropy passwords, and none of these tokens is one, so the fast hash
//! is used deliberately: Q78 in QUESTIONS.md.)

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use isms_core::kinds::ClientKind;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Name of the session cookie.
pub const SESSION_COOKIE: &str = "isms_session";
/// CSRF header every state-changing web request must carry (TDD 10.2).
pub const CSRF_HEADER: &str = "x-requested-with";
pub const CSRF_VALUE: &str = "isms";
/// API keys look like `isms_<prefix>.<secret>`.
pub const API_KEY_PREFIX: &str = "isms_";

pub const MAGIC_LINK_TTL: Duration = Duration::minutes(15);
pub const SESSION_TTL: Duration = Duration::days(30);

/// 32 random bytes, URL-safe base64.
#[must_use]
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// A short random id, safe for URLs and human eyes (invite codes, key prefixes).
#[must_use]
pub fn short_id(len: usize) -> String {
    const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut bytes = vec![0u8; len];
    rand::rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char)
        .collect()
}

#[must_use]
pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// A freshly minted API key and what the store keeps of it.
#[derive(Debug)]
pub struct MintedKey {
    pub key: String,
    pub prefix: String,
    pub hash: Vec<u8>,
}

#[must_use]
pub fn mint_api_key() -> MintedKey {
    let prefix = short_id(8);
    let secret = random_token();
    let key = format!("{API_KEY_PREFIX}{prefix}.{secret}");
    MintedKey {
        hash: hash_token(&key),
        key,
        prefix,
    }
}

/// Split `isms_<prefix>.<secret>` into its prefix.
fn key_prefix(key: &str) -> Option<&str> {
    let rest = key.strip_prefix(API_KEY_PREFIX)?;
    let (prefix, _) = rest.split_once('.')?;
    Some(prefix)
}

/// How the caller authenticated; decides the `client_kind` stamped on commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Client {
    Web,
    /// A key is scoped to one citizen in one society.
    ApiKey {
        key_id: i64,
        society_id: i64,
        citizen_id: u32,
    },
}

/// An authenticated request.
#[derive(Clone, Debug)]
pub struct Auth {
    pub account_id: i64,
    pub client: Client,
}

impl Auth {
    #[must_use]
    pub fn client_kind(&self) -> ClientKind {
        match self.client {
            Client::Web => ClientKind::Web,
            Client::ApiKey { .. } => ClientKind::ApiKey,
        }
    }
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        if let Some(value) = parts.headers.get(axum::http::header::AUTHORIZATION) {
            let value = value.to_str().map_err(|_| ApiError::Unauthorized)?;
            let key = value
                .strip_prefix("Bearer ")
                .ok_or(ApiError::Unauthorized)?
                .trim();
            let prefix = key_prefix(key).ok_or(ApiError::Unauthorized)?;
            let row = state
                .store
                .api_key_by_prefix(prefix)
                .await?
                .ok_or(ApiError::Unauthorized)?;
            if row.revoked_at.is_some() || row.key_hash != hash_token(key) {
                return Err(ApiError::Unauthorized);
            }
            return Ok(Auth {
                account_id: row.account_id,
                client: Client::ApiKey {
                    key_id: row.id,
                    society_id: row.society_id,
                    citizen_id: u32::try_from(row.citizen_id).unwrap_or(u32::MAX),
                },
            });
        }
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .unwrap_or_default();
        let token = jar.get(SESSION_COOKIE).ok_or(ApiError::Unauthorized)?;
        let account_id = state
            .store
            .session_account(&hash_token(token.value()))
            .await?
            .ok_or(ApiError::Unauthorized)?;
        // CSRF (TDD 10.2): a browser session may only change state with the header.
        let safe = matches!(
            parts.method,
            axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
        );
        if !safe {
            let ok = parts
                .headers
                .get(CSRF_HEADER)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v == CSRF_VALUE);
            if !ok {
                return Err(ApiError::Forbidden(format!(
                    "state-changing requests need the {CSRF_HEADER}: {CSRF_VALUE} header"
                )));
            }
        }
        Ok(Auth {
            account_id,
            client: Client::Web,
        })
    }
}

/// The caller's address for per-IP limits; `unknown` when the listener
/// carries no connect info (tests).
#[derive(Clone, Debug)]
pub struct ClientIp(pub String);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let forwarded = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim().to_owned());
        let ip = forwarded.or_else(|| {
            parts
                .extensions
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|c| c.0.ip().to_string())
        });
        Ok(ClientIp(ip.unwrap_or_else(|| "unknown".to_owned())))
    }
}

/// When a link or session minted now expires.
#[must_use]
pub fn expiry(ttl: Duration) -> chrono::DateTime<Utc> {
    Utc::now() + ttl
}
