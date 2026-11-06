use std::{
    collections::{HashMap, VecDeque},
    env,
    net::IpAddr,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::{errors::AppError, repository::TaskRepository};

const LOGIN_ATTEMPT_LIMIT: usize = 10;
const LOGIN_ATTEMPT_WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_RATE_LIMIT_KEYS: usize = 10_000;

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
    pub trust_proxy_headers: bool,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let environment = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
        let development = environment == "development";
        let cookie_name =
            env::var("SESSION_COOKIE_NAME").unwrap_or_else(|_| "nexora_session".to_string());
        if cookie_name.is_empty()
            || !cookie_name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(
                "SESSION_COOKIE_NAME must contain only ASCII letters, digits, '-' or '_'".into(),
            );
        }

        let cookie_secure = match env::var("SESSION_COOKIE_SECURE") {
            Ok(value) => value.parse::<bool>()?,
            Err(env::VarError::NotPresent) => !development,
            Err(error) => return Err(error.into()),
        };
        if !development && !cookie_secure {
            return Err("SESSION_COOKIE_SECURE can only be false when APP_ENV=development".into());
        }

        let session_ttl_hours = match env::var("SESSION_TTL_HOURS") {
            Ok(value) => value.parse::<i64>()?,
            Err(env::VarError::NotPresent) => 24,
            Err(error) => return Err(error.into()),
        };
        if !(1..=720).contains(&session_ttl_hours) {
            return Err("SESSION_TTL_HOURS must be between 1 and 720".into());
        }

        let trust_proxy_headers = match env::var("TRUST_PROXY_HEADERS") {
            Ok(value) => value.parse::<bool>()?,
            Err(env::VarError::NotPresent) => false,
            Err(error) => return Err(error.into()),
        };

        Ok(Self {
            cookie_name,
            cookie_secure,
            session_ttl_hours,
            trust_proxy_headers,
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tasks: TaskRepository,
    pub auth: AuthConfig,
    pub dummy_password_hash: String,
    login_attempts: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
}

impl AppState {
    pub fn new(pool: PgPool, auth: AuthConfig, dummy_password_hash: String) -> Self {
        Self {
            tasks: TaskRepository::new(pool.clone()),
            pool,
            auth,
            dummy_password_hash,
            login_attempts: Arc::default(),
        }
    }

    pub async fn begin_login_attempt(&self, ip: IpAddr) -> Result<(), AppError> {
        let now = Instant::now();
        let mut attempts = self.login_attempts.lock().await;
        if !attempts.contains_key(&ip) && attempts.len() >= MAX_RATE_LIMIT_KEYS {
            attempts.retain(|_, history| {
                history
                    .back()
                    .is_some_and(|instant| now.duration_since(*instant) < LOGIN_ATTEMPT_WINDOW)
            });
        }
        if !attempts.contains_key(&ip) && attempts.len() >= MAX_RATE_LIMIT_KEYS {
            return Err(AppError::too_many_requests(
                "LOGIN_RATE_LIMITED",
                "Too many login attempts. Try again later.",
            ));
        }

        let history = attempts.entry(ip).or_default();
        history.retain(|instant| now.duration_since(*instant) < LOGIN_ATTEMPT_WINDOW);
        if history.len() >= LOGIN_ATTEMPT_LIMIT {
            return Err(AppError::too_many_requests(
                "LOGIN_RATE_LIMITED",
                "Too many login attempts. Try again later.",
            ));
        }
        history.push_back(now);
        Ok(())
    }

    pub async fn clear_login_failures(&self, ip: IpAddr) {
        self.login_attempts.lock().await.remove(&ip);
    }

    pub fn login_client_ip(
        &self,
        remote: std::net::SocketAddr,
        headers: &axum::http::HeaderMap,
    ) -> IpAddr {
        resolve_client_ip(self.auth.trust_proxy_headers, remote, headers)
    }
}

fn resolve_client_ip(
    trust_proxy_headers: bool,
    remote: std::net::SocketAddr,
    headers: &axum::http::HeaderMap,
) -> IpAddr {
    if trust_proxy_headers
        && let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| IpAddr::from_str(value).ok())
    {
        return ip;
    }
    remote.ip()
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::http::{HeaderMap, HeaderValue};

    use super::{LOGIN_ATTEMPT_LIMIT, LOGIN_ATTEMPT_WINDOW, resolve_client_ip};

    #[test]
    fn login_limit_is_bounded_and_windowed() {
        assert_eq!(LOGIN_ATTEMPT_LIMIT, 10);
        assert_eq!(LOGIN_ATTEMPT_WINDOW.as_secs(), 900);
    }

    #[test]
    fn proxy_ip_is_used_only_when_explicitly_trusted() {
        let remote = "10.0.0.5:3000".parse::<SocketAddr>().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("198.51.100.8"));

        assert_eq!(
            resolve_client_ip(true, remote, &headers).to_string(),
            "198.51.100.8"
        );
        assert_eq!(
            resolve_client_ip(false, remote, &headers).to_string(),
            "10.0.0.5"
        );
    }

    #[test]
    fn invalid_proxy_ip_falls_back_to_peer_address() {
        let remote = "10.0.0.5:3000".parse::<SocketAddr>().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("not-an-ip"));

        assert_eq!(
            resolve_client_ip(true, remote, &headers).to_string(),
            "10.0.0.5"
        );
    }
}
