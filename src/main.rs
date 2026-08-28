use std::{env, net::SocketAddr, path::PathBuf};

use tokio::net::TcpListener;
use tracing::info;
use webhook_quiet_hours::{build_app, spawn_maintenance, AppConfig, AppState};

#[tokio::main]
async fn main() -> anyhowless::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("webhook_quiet_hours=info,tower_http=info")
    });
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .init();

    let config = AppConfig::from_env()?;
    info!(
        admin_token_source = config.admin_token_source(),
        encryption_key_source = config.encryption_key_source(),
        secret_directory = %config.secret_directory().display(),
        "runtime secrets resolved"
    );
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let dist = PathBuf::from(env::var("DIST_DIR").unwrap_or_else(|_| "dist".into()));
    let state = AppState::connect(&config).await?;
    spawn_maintenance(state.clone());
    let app = build_app(state, dist);
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    info!(port, "webhook quiet hours listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ctrl-c handler") };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}

mod anyhowless {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
