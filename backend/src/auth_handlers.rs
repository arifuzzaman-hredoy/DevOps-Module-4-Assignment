use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    auth::{
        AuthenticatedUser, UserProfile, hash_password, hash_token, new_token, normalize_email,
        session_cookie, validate_password, verify_password,
    },
    errors::AppError,
    state::AppState,
};

#[derive(Deserialize)]
pub struct Credentials {
    email: String,
    password: String,
}

#[derive(FromRow)]
struct LoginUser {
    id: Uuid,
    email: String,
    password_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct UserResponse {
    user: UserProfile,
}

#[derive(Serialize)]
pub struct CsrfResponse {
    csrf_token: String,
}

#[derive(Serialize)]
struct LogoutResponse {
    logged_out: bool,
}

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Credentials>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let Json(payload) = payload?;
    let email = normalize_email(&payload.email)?;
    validate_password(&payload.password)?;
    let password_hash = hash_password(payload.password).await?;
    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let session_token = new_token();
    let csrf_token = new_token();
    let token_hash = hash_token(session_token.as_bytes());
    let old_token = super::auth::cookie_value(&headers, &state.auth.cookie_name)
        .map(|token| hash_token(token.as_bytes()));

    let mut transaction = state.pool.begin().await.map_err(database_error)?;
    let user = sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, email, created_at
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| {
        if is_unique_violation(&error) {
            AppError::conflict(
                "EMAIL_ALREADY_REGISTERED",
                "An account with this email already exists",
            )
        } else {
            database_error(error)
        }
    })?;
    replace_session(
        &mut transaction,
        user_id,
        session_id,
        token_hash.as_slice(),
        &csrf_token,
        state.auth.session_ttl_hours,
        old_token.as_ref().map(|hash| hash.as_slice()),
    )
    .await?;
    transaction.commit().await.map_err(database_error)?;

    let profile = UserProfile {
        id: user.try_get("id").map_err(database_error)?,
        email: user.try_get("email").map_err(database_error)?,
        created_at: user.try_get("created_at").map_err(database_error)?,
    };
    let mut response = (StatusCode::CREATED, Json(UserResponse { user: profile })).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, session_cookie(&state, Some(&session_token))?);
    Ok(response)
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<Credentials>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let ip = state.login_client_ip(address, &headers);
    state.begin_login_attempt(ip).await?;
    let Json(payload) = payload?;
    let normalized_email = payload.email.trim().to_lowercase();
    let user = if normalize_email(&payload.email).is_ok() {
        sqlx::query_as::<_, LoginUser>(
            "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(&normalized_email)
        .fetch_optional(&state.pool)
        .await
        .map_err(database_error)?
    } else {
        None
    };

    let encoded_hash = user
        .as_ref()
        .map(|row| row.password_hash.clone())
        .unwrap_or_else(|| state.dummy_password_hash.clone());
    let password_matches = verify_password(payload.password, encoded_hash).await?;
    if user.is_none() || !password_matches {
        return Err(AppError::authentication_failed());
    }

    state.clear_login_failures(ip).await;
    let user = user.ok_or_else(AppError::authentication_failed)?;
    let user_id = user.id;
    let profile = UserProfile {
        id: user_id,
        email: user.email,
        created_at: user.created_at,
    };
    let session_id = Uuid::new_v4();
    let session_token = new_token();
    let csrf_token = new_token();
    let token_hash = hash_token(session_token.as_bytes());
    let old_token = super::auth::cookie_value(&headers, &state.auth.cookie_name)
        .map(|token| hash_token(token.as_bytes()));

    let mut transaction = state.pool.begin().await.map_err(database_error)?;
    replace_session(
        &mut transaction,
        user_id,
        session_id,
        token_hash.as_slice(),
        &csrf_token,
        state.auth.session_ttl_hours,
        old_token.as_ref().map(|hash| hash.as_slice()),
    )
    .await?;
    transaction.commit().await.map_err(database_error)?;

    let mut response = Json(UserResponse { user: profile }).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, session_cookie(&state, Some(&session_token))?);
    Ok(response)
}

pub async fn current_user(user: AuthenticatedUser) -> Json<UserResponse> {
    Json(UserResponse { user: user.into() })
}

pub async fn csrf_token(user: AuthenticatedUser) -> Json<CsrfResponse> {
    Json(CsrfResponse {
        csrf_token: user.csrf_token,
    })
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    _user: AuthenticatedUser,
) -> Result<Response, AppError> {
    if let Some(token) = super::auth::cookie_value(&headers, &state.auth.cookie_name) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_token(token.as_bytes()).as_slice())
            .execute(&state.pool)
            .await
            .map_err(database_error)?;
    }

    let mut response = Json(LogoutResponse { logged_out: true }).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, session_cookie(&state, None)?);
    Ok(response)
}

async fn replace_session(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    session_id: Uuid,
    token_hash: &[u8],
    csrf_token: &str,
    ttl_hours: i64,
    old_token_hash: Option<&[u8]>,
) -> Result<(), AppError> {
    if let Some(old_token_hash) = old_token_hash {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(old_token_hash)
            .execute(&mut **transaction)
            .await
            .map_err(database_error)?;
    }
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, token_hash, csrf_token, expires_at)
        VALUES ($1, $2, $3, $4, NOW() + ($5::bigint * INTERVAL '1 hour'))
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(token_hash)
    .bind(csrf_token)
    .bind(ttl_hours)
    .execute(&mut **transaction)
    .await
    .map_err(database_error)?;
    sqlx::query("DELETE FROM sessions WHERE expires_at <= NOW()")
        .execute(&mut **transaction)
        .await
        .map_err(database_error)?;
    Ok(())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .is_some_and(|code| code == "23505")
}

fn database_error(error: sqlx::Error) -> AppError {
    tracing::error!("Authentication database operation failed: {}", error);
    AppError::internal("Authentication service unavailable")
}
