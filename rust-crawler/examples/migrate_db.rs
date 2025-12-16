use sqlx::postgres::PgPoolOptions;
use sqlx::ConnectOptions;
use std::env;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("🔌 Connecting to DB..."); 

    let opts = sqlx::postgres::PgConnectOptions::from_url(&db_url.parse().unwrap())
        .expect("Invalid DATABASE_URL")
        .statement_cache_capacity(0);

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|conn, _meta| Box::pin(async move {
            use sqlx::Executor;
            conn.execute("DEALLOCATE ALL").await.map(|_| ())
        }))
        .connect_with(opts)
        .await?;

    println!("✅ Connected! Applying Migration...");

    match sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS marketing_data JSONB;").execute(&pool).await {
        Ok(_) => println!("✅ Migration Success: 'marketing_data' column added (or already existed)."),
        Err(e) => println!("❌ Migration Failed: {}", e),
    }

    Ok(())
}
