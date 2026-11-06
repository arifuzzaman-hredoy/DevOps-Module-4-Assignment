mod auth;
mod auth_handlers;
mod db;
mod errors;
mod handlers;
mod models;
mod repository;
mod router;
mod state;

use std::net::SocketAddr;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Nexora API");

    let pool = db::create_pool().await?;
    tracing::info!("PostgreSQL connection pool initialized");

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations completed successfully");

    let auth_config = state::AuthConfig::from_env()?;
    let dummy_password_hash = auth::hash_password("invalid-login-timing-placeholder".to_string())
        .await
        .map_err(|_| "Could not initialize authentication")?;
    let app_state = state::AppState::new(pool.clone(), auth_config, dummy_password_hash);
    tokio::spawn(session_cleanup(pool));

    let app = router::build_router(app_state);

    let address = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!("Nexora API listening on {}", address);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn session_cleanup(pool: sqlx::PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
    loop {
        interval.tick().await;
        if let Err(error) = sqlx::query("DELETE FROM sessions WHERE expires_at <= NOW()")
            .execute(&pool)
            .await
        {
            tracing::error!("Expired session cleanup failed: {}", error);
        }
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if let Err(error) = result {
                            tracing::error!("Failed to listen for Ctrl-C: {}", error);
                        }
                    }
                    _ = terminate.recv() => {}
                }
            }
            Err(error) => {
                tracing::error!("Failed to listen for SIGTERM: {}", error);
                if let Err(error) = tokio::signal::ctrl_c().await {
                    tracing::error!("Failed to listen for Ctrl-C: {}", error);
                }
            }
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to listen for Ctrl-C: {}", error);
    }

    tracing::info!("Shutdown signal received; draining active requests");
}
