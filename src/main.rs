use std::fmt::Display;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use maud::{html, Markup, Render};
use sqlx::SqlitePool;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tracing::{error, info};
use tracing_subscriber::prelude::*;

mod db;
mod handler;
mod html;

#[derive(Clone)]
struct AppState {
    db: db::Db,
}

#[derive(Debug)]
pub struct Session {
    user_id: db::UserId,
    session_id: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "matrafl=debug,sqlx=warn,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .try_init()
        .unwrap();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let db_options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_url)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .create_if_missing(true);
    let db_pool = SqlitePool::connect_with(db_options).await.unwrap();

    sqlx::migrate!("./migrations").run(&db_pool).await.unwrap();

    if let Some(arg1) = std::env::args().nth(1) {
        if arg1 == "create-user" {
            let username = std::env::args().nth(2).expect("Missing username argument");
            let password = rpassword::prompt_password("Password: ").unwrap();

            db::Db::new(db_pool.clone())
                .create_user(&username, &password)
                .await
                .unwrap();

            return;
        }
    }

    let assets_path = std::env::var("ASSETS_PATH").unwrap();

    let port = std::env::var("PORT").unwrap().parse::<u16>().unwrap();

    let db = db::Db::new(db_pool.clone());

    let app_state = AppState { db };

    let app = Router::new()
        .route("/", routing::get(handler::index))
        .route("/days/{date}", routing::get(handler::days_read))
        .route("/consumptions", routing::post(handler::consumptions_create))
        .route(
            "/consumptions/{id}",
            routing::get(handler::consumptions_read),
        )
        .route(
            "/consumptions/{id}",
            routing::post(handler::consumptions_update),
        )
        .route(
            "/consumptions/{id}/delete",
            routing::post(handler::consumptions_delete),
        )
        .route("/weights", routing::get(handler::weights_index))
        .route("/weights", routing::post(handler::weights_create))
        .route("/weights/{id}", routing::get(handler::weights_read))
        .route("/weights/{id}", routing::post(handler::weights_update))
        .route(
            "/weights/{id}/delete",
            routing::post(handler::weights_delete),
        )
        .route("/foods", routing::get(handler::foods_index))
        .route("/foods", routing::post(handler::foods_create))
        .route("/foods/{id}", routing::get(handler::foods_read))
        .route("/foods/{id}", routing::post(handler::foods_update))
        .route("/foods/{id}/delete", routing::post(handler::foods_delete))
        .route("/recipes", routing::get(handler::recipes_index))
        .route("/recipes", routing::post(handler::recipes_create))
        .route("/recipes/{id}", routing::get(handler::recipes_read))
        .route("/recipes/{id}", routing::post(handler::recipes_update))
        .route(
            "/recipes/{id}/delete",
            routing::post(handler::recipes_delete),
        )
        .route("/ingredients", routing::post(handler::ingredients_create))
        .route("/ingredients/{id}", routing::get(handler::ingredients_read))
        .route(
            "/ingredients/{id}",
            routing::post(handler::ingredients_update),
        )
        .route(
            "/ingredients/{id}/delete",
            routing::post(handler::ingredients_delete),
        )
        .route("/account", routing::get(handler::account_read))
        .route("/account/login", routing::get(handler::account_login_form))
        .route("/account/login", routing::post(handler::account_login))
        .route("/account/logout", routing::post(handler::account_logout))
        .route("/account/export", routing::post(handler::account_export))
        .nest_service("/assets", ServeDir::new(assets_path))
        .with_state(app_state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let sched = JobScheduler::new().await.unwrap();
    sched
        .add(
            Job::new_async("1/7 * * * * *", move |_uuid, _l| {
                let sched_db = db::Db::new(db_pool.clone());
                Box::pin(async move {
                    sched_db.delete_expired_sessions().await.unwrap();
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();
    sched.start().await.unwrap();

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    info!(addr = ?listener.local_addr().unwrap(), "starting");
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug)]
enum AppError {
    InvalidDate,
    SQLError,
    HTTPError,
    NoSessionCookie,
    UnknownSessionId,
    PasswordHashError,
    Forbidden,
    InvalidConsumableType,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NoSessionCookie => redirect_to(AppUrl::AccountLogin),
            AppError::UnknownSessionId => redirect_to(AppUrl::AccountLogin),
            AppError::InvalidDate => (
                StatusCode::BAD_REQUEST,
                Html(html::error_page().into_string()),
            )
                .into_response(),
            e => {
                error!(error = ?e, "app error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(html::error_page().into_string()),
                )
                    .into_response()
            }
        }
    }
}

impl From<chrono::ParseError> for AppError {
    fn from(e: chrono::ParseError) -> Self {
        error!(error = ?e, "Chrono error");
        AppError::InvalidDate
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        error!(error = ?e, "SQL error");
        AppError::SQLError
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        error!(error = ?e, "Hash error");
        AppError::PasswordHashError
    }
}

impl From<axum::http::Error> for AppError {
    fn from(e: axum::http::Error) -> Self {
        error!(error = ?e, "HTTP error");
        AppError::HTTPError
    }
}

impl FromRequestParts<AppState> for Session {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match CookieJar::from_headers(&parts.headers)
            .get("MATRAFL_SESSION")
            .map(Cookie::value)
        {
            Some(session_id) => match state.db.get_session_user_id(session_id).await? {
                Some(user_id) => Ok(Session {
                    user_id: db::UserId(user_id),
                    session_id: session_id.to_string(),
                }),
                None => Err(AppError::UnknownSessionId),
            },
            None => Err(AppError::NoSessionCookie),
        }
    }
}

impl FromRequestParts<AppState> for Option<Session> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match CookieJar::from_headers(&parts.headers)
            .get("MATRAFL_SESSION")
            .map(Cookie::value)
        {
            Some(session_id) => match state.db.get_session_user_id(session_id).await? {
                Some(user_id) => Ok(Some(Session {
                    user_id: db::UserId(user_id),
                    session_id: session_id.to_string(),
                })),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }
}

enum AppUrl {
    Home,
    DaySummary(chrono::NaiveDate),
    Consumptions,
    ConsumptionsId(String),
    ConsumptionsIdDelete(String),
    Weights,
    WeightsId(String),
    WeightsIdDelete(String),
    Foods,
    FoodsId(String),
    FoodsIdDelete(String),
    Recipes,
    RecipesId(String),
    RecipesIdDelete(String),
    Ingredients,
    IngredientsId(String),
    IngredientsIdDelete(String),
    Account,
    AccountLogin,
    AccountLogout,
    AccountExport,
}

impl Render for AppUrl {
    fn render(&self) -> Markup {
        html! {
            (self.to_string())
        }
    }
}

impl Display for AppUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AppUrl::Home => "/".to_string(),
                AppUrl::DaySummary(date) => format!("/days/{}", date),
                AppUrl::Consumptions => "/consumptions".to_string(),
                AppUrl::ConsumptionsId(id) => format!("/consumptions/{}", id),
                AppUrl::ConsumptionsIdDelete(id) => format!("/consumptions/{}/delete", id),
                AppUrl::Weights => "/weights".to_string(),
                AppUrl::WeightsId(id) => format!("/weights/{}", id),
                AppUrl::WeightsIdDelete(id) => format!("/weights/{}/delete", id),
                AppUrl::Foods => "/foods".to_string(),
                AppUrl::FoodsId(id) => format!("/foods/{}", id),
                AppUrl::FoodsIdDelete(id) => format!("/foods/{}/delete", id),
                AppUrl::Recipes => "/recipes".to_string(),
                AppUrl::RecipesId(id) => format!("/recipes/{}", id),
                AppUrl::RecipesIdDelete(id) => format!("/recipes/{}/delete", id),
                AppUrl::Ingredients => "/ingredients".to_string(),
                AppUrl::IngredientsId(id) => format!("/ingredients/{}", id),
                AppUrl::IngredientsIdDelete(id) => format!("/ingredients/{}/delete", id),
                AppUrl::Account => "/account".to_string(),
                AppUrl::AccountLogin => "/account/login".to_string(),
                AppUrl::AccountLogout => "/account/logout".to_string(),
                AppUrl::AccountExport => "/account/export".to_string(),
            }
        )
    }
}

fn redirect_to(url: AppUrl) -> Response {
    Redirect::to(url.to_string().as_str()).into_response()
}
