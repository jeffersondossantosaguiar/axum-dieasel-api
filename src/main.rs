use std::net::SocketAddr;

use deadpool_diesel::postgres::{Manager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::config;
use crate::errors::internal_error;
use crate::routes::app_router;

// Import modules
mod config;
mod domain;
mod errors;
mod handlers;
mod infra;
mod routes;

// Define embedded database migrations
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

// Struct to hold the application state
#[derive(Clone)]
pub struct AppState {
    pool: Pool,
}

// Main function, the entry point of the application
#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    init_tracing();

    // Load configuration settings
    let config = config().await;

    // Create a connection pool to the PostgresSQL database
    let manager = Manager::new(
        config.db_url().to_string(),
        deadpool_diesel::Runtime::Tokio1,
    );
    let pool = Pool::builder(manager).build().unwrap();

    // Apply pending database migrations
    run_migrations(&pool).await;

    // Create an instance of the application state
    let state = AppState { pool };

    // Configure the application router
    let app = app_router(state.clone()).with_state(state);

    // Define the host and port for the server
    let host = config.server_host();
    let port = config.server_port();

    let address = format!("{}:{}", host, port);

    // Parse the socket address
    let socket_addr: SocketAddr = address.parse().unwrap();

    // Log the server's listening address
    tracing::info!("listening on http://{}", socket_addr);

    // Start the Axum server
    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .await
        .map_err(internal_error)
        .unwrap()
}

// Function to initialize tracing for logging
fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_tokio_postgres=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// Function to run database migrations
async fn run_migrations(pool: &Pool) {
    let conn = pool.get().await.unwrap();
    conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
        .await
        .unwrap()
        .unwrap();
}