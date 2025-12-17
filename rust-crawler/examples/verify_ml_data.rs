use sqlx::postgres::PgPoolOptions;
use sqlx::ConnectOptions;
use sqlx::Row;
use std::env;
use dotenv::dotenv;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("🔌 Connecting to DB..."); 

    // Supabase Fix
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

    println!("✅ Connected! Fetching recent tasks with ML data...");

    // Query mostly focused on ML columns
    let rows = sqlx::query(
        r#"
        SELECT keyword, sentiment, entities, category, marketing_data 
        FROM tasks 
        WHERE status = 'completed' 
        ORDER BY created_at DESC 
        LIMIT 5
        "#
    )
    .fetch_all(&pool)
    .await?;

    if rows.is_empty() {
        println!("⚠️ No completed tasks found. Run a crawl first to see data.");
        return Ok(());
    }

    for (i, row) in rows.iter().enumerate() {
        let keyword: String = row.get("keyword");
        let sentiment: Option<String> = row.get("sentiment");
        let category: Option<String> = row.get("category");
        let entities: Option<Value> = row.get("entities");
        let marketing: Option<Value> = row.get("marketing_data");

        println!("\n--- Result #{} (Keyword: '{}') ---", i + 1, keyword);
        println!("🧠 AI Sentiment:  {}", sentiment.unwrap_or("None".to_string()));
        println!("🏷️  ML Category:   {}", category.unwrap_or("None".to_string()));
        println!("👤 NER Entities:  {}", entities.unwrap_or(Value::Null));
        println!("📢 Marketing Data: {}", marketing.unwrap_or(Value::Null)); 
    }

    Ok(())
}
