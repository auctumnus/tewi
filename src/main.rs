use askama::Template;
use axum::{
    Extension, Router,
    extract::{DefaultBodyLimit, State, connect_info::IntoMakeServiceWithConnectInfo},
    http::StatusCode,
    response::Html,
    routing::{delete, get, post},
};

use sqlx::PgPool;
use uuid::Uuid;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    board_info::BoardInfo,
    config::{AdminCommand, BoardCategoryCommand, BoardCommand, CliAction},
    err::AppError,
    extract_session::AdminSession,
    models::{
        admins::Admin, board_categories::{BoardCategory, BoardCategoryRepository}, boards::{Board, BoardRepository, CreateBoard}
    },
};

mod auth;
mod board_info;
mod config;
mod controllers;
mod err;
mod extract_session;
mod filters;
mod middleware;
mod models;
mod pagination;
mod parse_multipart;
mod view_structs;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    categories: Vec<BoardCategory>,
    boards: Vec<Board>,
}

/// Shared connections, config, etc.
/// Should be cheap to clone.
#[derive(Clone)]
pub struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = config::CONFIG.clone();

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

    let state = AppState { db: pool };

    let fake_admin = Admin {
        id: Uuid::nil(),
        name: "cli".to_string(),
        password_hash: "".to_string(),
        created_at: chrono::Utc::now(),
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
        CliAction::Admin(admin_command) => match admin_command {
            AdminCommand::List => {
                let repo = models::admins::AdminRepository::new(&state);
                let admins = repo.list_all().await?;
                for admin in admins {
                    println!("Admin: {:?}", admin);
                }
                Ok(())
            }
            AdminCommand::New { name, password } => {
                let repo = models::admins::AdminRepository::new(&state);
                let admin = repo.create(&name, &password).await?;
                println!("Created admin: {:?}", admin);
                Ok(())
            }
            AdminCommand::Delete { name } => {
                let repo = models::admins::AdminRepository::new(&state);
                repo.delete_by_name(&name).await?;
                println!("Deleted admin: {}", name);
                Ok(())
            }
            AdminCommand::ChangePassword { name, new_password } => {
                let repo = models::admins::AdminRepository::new(&state);
                repo.change_password(&name, &new_password).await?;
                println!("Changed password for admin: {}", name);
                Ok(())
            }
        },
        CliAction::Board(board_command) => match board_command {
            BoardCommand::New { name, slug, description, category } => {
                let repo = models::boards::BoardRepository::new(&state);
                let category_id = if let Some(cat_name) = &category {
                    let category_repo = models::board_categories::BoardCategoryRepository::new(&state);
                    if let Some(cat) = category_repo.find_by_name(cat_name).await? {
                        Some(cat.id)
                    } else {
                        println!("Category '{}' not found", cat_name);
                        return Ok(());
                    }
                } else {
                    None
                };
                let create = CreateBoard {
                    name,
                    description,
                    slug,
                    category_id,
                };
                let board = repo.create(fake_admin, create).await?;
                println!("Created board: {:?}", board);
                Ok(())
            }
            BoardCommand::List => {
                let repo = models::boards::BoardRepository::new(&state);
                let boards = repo.list_all().await?;
                if boards.is_empty() {
                    println!("No boards found.");
                    return Ok(());
                }
                let category_repo = models::board_categories::BoardCategoryRepository::new(&state);
                for board in boards {
                    println!("Board: {:?}", board);
                    if let Some(category_id) = board.category_id &&
                        let Ok(category) = category_repo.find_by_id(category_id).await {
                            println!("  Category: {:?}", category);
                        
                    }
                }
                Ok(())
            }
            BoardCommand::Delete { name } => {
                let repo = models::boards::BoardRepository::new(&state);
                repo.delete_by_name(fake_admin, &name).await?;
                println!("Deleted board: {}", name);
                Ok(())
            }
        },
        CliAction::Category(category_command) => match category_command {
            BoardCategoryCommand::New { name } => {
                let repo = models::board_categories::BoardCategoryRepository::new(&state);
                let category = repo.create(fake_admin, name).await?;
                println!("Created category: {:?}", category);
                Ok(())
            }
            BoardCategoryCommand::List => {
                let repo = models::board_categories::BoardCategoryRepository::new(&state);
                let categories = repo.list_all().await?;
                if categories.is_empty() {
                    println!("No categories found.");
                    return Ok(());
                }
                for category in categories {
                    println!("Category: {:?}", category);
                }
                Ok(())
            }
            BoardCategoryCommand::Delete { name } => {
                let repo = models::board_categories::BoardCategoryRepository::new(&state);

                repo.delete_by_name(fake_admin, &name).await?;
                println!("Deleted category: {}", name);
                Ok(())
            }
        },
        CliAction::Clean => {
            println!("Clearing expired sessions...");
            let sessions = models::sessions::SessionRepository::new(&state);
            sessions.delete_expired().await?;

            println!("Cleaned up the database");
            Ok(())
        }
    }
}

fn create_router(state: AppState) -> IntoMakeServiceWithConnectInfo<Router, SocketAddr> {
    let admin_router = Router::new()
        .layer(Extension(AdminSession))
        .route("/login", get(controllers::admin::login_page))
        .route("/login", post(controllers::admin::login))
        .route("/logout", post(controllers::admin::logout))
        .route("/boards", get(controllers::admin::boards))
        .route(
            "/boards/create",
            get(controllers::admin::display_create_board),
        )
        .route("/boards/create", post(controllers::admin::create_board))
        .route("/boards/board/{slug}", get(controllers::admin::view_board))
        .route(
            "/boards/board/{slug}",
            post(controllers::admin::update_board),
        )
        .route(
            "/boards/board/{slug}/delete",
            post(controllers::admin::delete_board),
        )
        .route("/bans", get(controllers::admin::bans))
        .route("/categories", get(controllers::admin::categories))
        .route(
            "/categories/create",
            get(controllers::admin::display_create_category),
        )
        .route(
            "/categories/create",
            post(controllers::admin::create_category),
        )
        .route(
            "/categories/category/{id}",
            get(controllers::admin::view_category),
        )
        .route(
            "/categories/edit/{id}",
            post(controllers::admin::update_category),
        )
        .route(
            "/categories/delete/{id}",
            delete(controllers::admin::delete_category),
        );

    Router::new()
        .layer(Extension(BoardInfo))
        .route("/", get(home))
        .route("/board/{slug}", get(controllers::thread::board_page))
        .route(
            "/board/{slug}",
            post(controllers::thread::create_thread).layer(DefaultBodyLimit::max(10485760)),
        )
        .route(
            "/board/{slug}/thread/{id}",
            get(controllers::thread::thread),
        )
        .route(
            "/board/{slug}/thread/{id}",
            post(controllers::thread::create_post).layer(DefaultBodyLimit::max(10485760)),
        )
        .nest("/admin", admin_router)
        .nest_service("/static", ServeDir::new("frontend/dist"))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
        .layer(axum::middleware::from_fn(
            middleware::pretty_errors::pretty_error_codes,
        ))
        .fallback(fallback_route)
        .with_state(state)
        .into_make_service_with_connect_info::<SocketAddr>()
}

async fn fallback_route() -> Result<Html<String>, StatusCode> {
    let html = (view_structs::status::error::not_found::NotFoundTemplate { board_name: None })
        .render()
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Html(html))
}

async fn home(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let category_repo = BoardCategoryRepository::new(&s);
    let boards_repo = BoardRepository::new(&s);

    let categories = category_repo
        .list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let boards = boards_repo
        .list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let html = (HomeTemplate { boards, categories })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
