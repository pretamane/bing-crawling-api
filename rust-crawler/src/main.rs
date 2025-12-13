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

use tower_http::services::ServeDir;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::trigger_crawl,
        api::get_crawl_status,
        api::list_tasks,
        api::list_proxies,
        api::add_proxy,
        api::remove_proxy,
        api::enable_proxy,
        api::proxy_stats
    ),
    components(
        schemas(
            api::CrawlRequest, 
            api::CrawlResponse, 
            api::TaskResult, 
            api::TaskSummary,
            api::AddProxyRequest,
            api::AddProxyResponse,
            api::RemoveProxyResponse,
            crate::proxy::ProxyInfo,
            crate::proxy::ProxyStats,
            crate::proxy::ProxyProtocol
        )
    ),
    tags(
        (name = "crawler", description = "Crawler Management API"),
        (name = "proxy", description = "Proxy Management API")
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
        .route("/tasks", get(api::list_tasks))
        // Proxy management endpoints
        .route("/proxies", get(api::list_proxies))
        .route("/proxies", post(api::add_proxy))
        .route("/proxies/:proxy_id", axum::routing::delete(api::remove_proxy))
        .route("/proxies/:proxy_id/enable", post(api::enable_proxy))
        .route("/proxies/stats", get(api::proxy_stats))
        .nest_service("/", ServeDir::new("static")) // Serve Dashboard
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
