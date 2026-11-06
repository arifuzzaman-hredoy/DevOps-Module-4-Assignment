use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{HeaderMap, HeaderValue, Method, request::Parts},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rand::{RngCore, rngs::OsRng};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{errors::AppError, state::AppState};

pub const CSRF_HEADER: &str = "x-csrf-token";

#[derive(Clone, Debug, FromRow)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    pub(crate) csrf_token: String,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<AuthenticatedUser> for UserProfile {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let token = cookie_value(&parts.headers, &state.auth.cookie_name)
            .ok_or_else(AppError::unauthorized)?;
        let hash = hash_token(token.as_bytes());
        let user = sqlx::query_as::<_, AuthenticatedUser>(
            r#"
            SELECT users.id, users.email, users.created_at, sessions.expires_at, sessions.csrf_token
            FROM sessions
            JOIN users ON users.id = sessions.user_id
            WHERE sessions.token_hash = $1
            "#,
        )
        .bind(hash.as_slice())
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| {
            tracing::error!("Session lookup failed: {}", error);
            AppError::internal("Authentication unavailable")
        })?
        .ok_or_else(AppError::unauthorized)?;

        if !session_is_active(user.expires_at, Utc::now()) {
            return Err(AppError::unauthorized());
        }

        if matches!(
            parts.method,
            Method::POST | Method::PUT | Method::PATCH | Method::DELETE
        ) {
            let provided = parts
                .headers
                .get(CSRF_HEADER)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if !csrf_matches(&user.csrf_token, provided) {
                return Err(AppError::forbidden(
                    "CSRF_TOKEN_INVALID",
                    "A valid CSRF token is required",
                ));
            }
        }

        Ok(user)
    }
}

fn session_is_active(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    expires_at > now
}

fn csrf_matches(expected: &str, provided: &str) -> bool {
    bool::from(expected.as_bytes().ct_eq(provided.as_bytes()))
}

pub fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    if email.len() > 254 || !is_valid_email(&email) {
        return Err(AppError::validation(
            "INVALID_EMAIL",
            "Enter a valid email address",
        ));
    }
    Ok(email)
}

fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.rsplit_once('@') else {
        return false;
    };
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-/=?^_`{|}~.".contains(&byte))
    {
        return false;
    }

    let labels: Vec<&str> = domain.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    let valid = (6..=128).contains(&password.len())
        && password.bytes().any(|byte| byte.is_ascii_lowercase())
        && password.bytes().any(|byte| byte.is_ascii_uppercase())
        && password.bytes().any(|byte| byte.is_ascii_digit())
        && password.bytes().any(|byte| !byte.is_ascii_alphanumeric());
    if !valid {
        return Err(AppError::validation(
            "WEAK_PASSWORD",
            "Use 6–128 bytes with an uppercase letter, lowercase letter, number, and symbol",
        ));
    }
    Ok(())
}

pub async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| ())
    })
    .await
    .map_err(|error| {
        tracing::error!("Password hashing task failed: {}", error);
        AppError::internal("Password processing failed")
    })?
    .map_err(|_| AppError::internal("Password processing failed"))
}

pub async fn verify_password(password: String, encoded_hash: String) -> Result<bool, AppError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&encoded_hash).map_err(|_| ())?;
        Ok::<bool, ()>(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
        )
    })
    .await
    .map_err(|error| {
        tracing::error!("Password verification task failed: {}", error);
        AppError::internal("Password processing failed")
    })?
    .map_err(|_| AppError::internal("Password processing failed"))
}

pub fn new_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &[u8]) -> [u8; 32] {
    Sha256::digest(token).into()
}

pub fn cookie_value<'a>(headers: &'a HeaderMap, cookie_name: &str) -> Option<&'a str> {
    headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(name, value)| (name == cookie_name).then_some(value))
}

pub fn session_cookie(state: &AppState, token: Option<&str>) -> Result<HeaderValue, AppError> {
    let max_age = if token.is_some() {
        state.auth.session_ttl_hours * 3600
    } else {
        0
    };
    let mut cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        state.auth.cookie_name,
        token.unwrap_or_default(),
        max_age
    );
    if state.auth.cookie_secure {
        cookie.push_str("; Secure");
    }
    HeaderValue::from_str(&cookie)
        .map_err(|_| AppError::internal("Could not configure session cookie"))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{
        csrf_matches, hash_password, hash_token, is_valid_email, new_token, normalize_email,
        session_is_active, validate_password, verify_password,
    };

    #[test]
    fn email_validation_and_normalization_are_consistent() {
        assert_eq!(
            normalize_email("  Person.Name@Example.COM ").unwrap(),
            "person.name@example.com"
        );
        for invalid in ["", "person", "a@b", "a..b@example.com", "@example.com"] {
            assert!(
                !is_valid_email(invalid),
                "accepted invalid email {invalid:?}"
            );
        }
    }

    #[test]
    fn password_policy_rejects_short_or_incomplete_passwords() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("Aa1!b").is_err());
        assert!(validate_password("Aa1!bc").is_ok());
        assert!(validate_password("alllowercase123!").is_err());
        assert!(validate_password("StrongPassword123!").is_ok());
    }

    #[test]
    fn session_tokens_are_random_and_only_their_hash_is_used_for_lookup() {
        let first = new_token();
        let second = new_token();
        assert_ne!(first, second);
        assert_ne!(hash_token(first.as_bytes()), hash_token(second.as_bytes()));
        assert_eq!(hash_token(first.as_bytes()).len(), 32);
    }

    #[tokio::test]
    async fn password_hashes_verify_without_storing_the_plaintext() {
        let password = "StrongPassword123!".to_string();
        let encoded = hash_password(password.clone()).await.unwrap();
        assert_ne!(encoded, password);
        assert!(encoded.starts_with("$argon2id$"));
        assert!(verify_password(password, encoded.clone()).await.unwrap());
        assert!(
            !verify_password("WrongPassword123!".to_string(), encoded)
                .await
                .unwrap()
        );
    }

    #[test]
    fn csrf_tokens_match_only_the_session_token() {
        assert!(csrf_matches("session-bound-token", "session-bound-token"));
        assert!(!csrf_matches("session-bound-token", "other-token"));
        assert!(!csrf_matches("session-bound-token", ""));
    }

    #[test]
    fn expired_sessions_are_not_active() {
        let now = Utc::now();
        assert!(session_is_active(now + Duration::seconds(1), now));
        assert!(!session_is_active(now - Duration::seconds(1), now));
    }
}
