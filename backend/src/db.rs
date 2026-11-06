use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{env, error::Error, str::FromStr};

pub async fn create_pool() -> Result<PgPool, Box<dyn Error + Send + Sync>> {
    let host = env::var("DB_HOST")?;
    let port: u16 = env::var("DB_PORT")?.parse()?;
    let database = env::var("DB_NAME")?;
    let username = env::var("DB_USER")?;
    let password = env::var("DB_PASSWORD")?;
    let ssl_mode = env::var("DB_SSL_MODE").unwrap_or_else(|_| "verify-full".to_string());
    let ssl_mode = PgSslMode::from_str(&ssl_mode)?;

    let mut options = PgConnectOptions::new()
        .host(&host)
        .port(port)
        .database(&database)
        .username(&username)
        .password(&password)
        .ssl_mode(ssl_mode);

    if let Ok(root_cert_path) = env::var("DB_SSL_ROOT_CERT") {
        options = options.ssl_root_cert(root_cert_path);
    }

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect_with(options)
        .await?;

    Ok(pool)
}
