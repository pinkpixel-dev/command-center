//! Starting and stopping the server.

use std::sync::Arc;

use command_center_server::auth::Auth;
use command_center_server::config::Config;
use command_center_server::credentials::EnvCredentials;
use command_center_server::routes;
use command_center_server::state::AppState;
use command_center_server::web;

#[tokio::main]
async fn main() {
    if let Err(message) = run().await {
        eprintln!("command-center-server: {message}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let config = Config::from_env()?;
    // Read before the library is opened, so a server with no password
    // configured stops before it has anything to serve.
    let auth = Arc::new(Auth::from_env()?);

    let state = AppState::new(
        config.clone(),
        Arc::new(EnvCredentials),
        auth,
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(|error| format!("could not open the library: {error}"))?;

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|error| format!("could not listen on {}: {error}", config.addr))?;

    println!(
        "Command Center {} is serving {} on http://{}",
        env!("CARGO_PKG_VERSION"),
        config.library_path().display(),
        config.addr
    );
    let codex = state.codex.clone();
    let app = match &config.web_dir {
        Some(dir) => {
            println!("Serving the web interface from {}", dir.display());
            web::serve(routes::router(state), dir)
        }
        None => {
            // Normal in development, where Vite serves the frontend and
            // proxies the API here. Said out loud so it is never a mystery.
            println!("No web bundle found. Serving the API only.");
            routes::router(state)
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| format!("the server stopped: {error}"))?;

    // Codex is a separate long-lived process. Without this it can outlive the
    // server that started it, which in a container means a stop that hangs.
    codex.shutdown().await;
    println!("Command Center stopped.");
    Ok(())
}

/// Ctrl-C for a terminal, SIGTERM for everything a container runtime does.
async fn shutdown_signal() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            // Without the signal handler there is still Ctrl-C, so this waits
            // rather than shutting the server down at startup.
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = interrupt => {}
        _ = terminate => {}
    }
}
