use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{net::SocketAddr, str::FromStr};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use clap::{Parser, Subcommand};

use crate::err::AppError;

mod models;
mod err;
mod auth;
mod pagination;
mod extract_session;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {}

/// Shared connections, config, etc.
/// Should be cheap to clone.
#[derive(Clone)]
pub struct AppState {
    db: PgPool,
}
#[derive(Debug, Clone, Subcommand)]
enum CliAction {
    /// Start the web server
    Serve,
    /// Create a new admin user
    NewAdmin {
        #[arg(long)]
        name: String,

        #[arg(long)]
        password: String,
    },
    /// Delete an admin user
    DeleteAdmin {
        #[arg(long)]
        name: String,
    },
    /// Clean up the database
    Clean,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "tewi", about = "A web application")]
struct Cli {
    #[command(subcommand)]
    action: Option<CliAction>,

    #[arg(short, long, default_value = "3000", global = true)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tewi=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://user:password@localhost/tewi".to_string());
    
    let pool = PgPool::connect(&database_url).await?;
    
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        db: pool
    };

    match cli.action.unwrap_or(CliAction::Serve) {
        CliAction::Serve => {
            let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
            let app = create_router(state.clone());

            tracing::debug!("listening on {}", addr);

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;

            Ok(())
        }
        CliAction::NewAdmin { name, password } => {
            let repo = models::admins::AdminRepository::new(&state);
            let admin = repo.create(&name, &password).await?;
            println!("Created admin: {:?}", admin);
            Ok(())
        }
        CliAction::DeleteAdmin { name } => {
            let repo = models::admins::AdminRepository::new(&state);
            repo.delete_by_name(&name).await?;
            println!("Deleted admin: {}", name);
            Ok(())
        }
        CliAction::Clean => {
            println!("Clearing expired sessions...");
            let sessions = models::sessions::SessionRepository::new(&state);
            sessions.delete_expired().await?;

            println!("Cleaned up the database");
            Ok(())
        }
    }
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .nest_service("/static", ServeDir::new("frontend/dist"))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

async fn home(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (HomeTemplate {}).render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
