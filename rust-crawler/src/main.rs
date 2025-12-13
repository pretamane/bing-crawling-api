mod api;
mod crawler;
mod db;
mod proxy;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use dotenv::dotenv;
use std::env;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::trigger_crawl,
        api::get_crawl_status
    ),
    components(
        schemas(api::CrawlRequest, api::CrawlResponse, api::TaskResult)
    ),
    tags(
        (name = "crawler", description = "Crawler Management API")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    db::init_db(&pool).await?;

    let state = Arc::new(api::AppState { pool });

    let app = Router::new()
        .merge(SwaggerUi::new("/rust-crawler-swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/crawl", post(api::trigger_crawl))
        .route("/crawl/:task_id", get(api::get_crawl_status))
        // Proxy management endpoints
        .route("/proxies", get(api::list_proxies))
        .route("/proxies", post(api::add_proxy))
        .route("/proxies/:proxy_id", axum::routing::delete(api::remove_proxy))
        .route("/proxies/:proxy_id/enable", post(api::enable_proxy))
        .route("/proxies/stats", get(api::proxy_stats))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
