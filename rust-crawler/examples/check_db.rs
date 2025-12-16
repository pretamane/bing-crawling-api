use sqlx::postgres::PgPoolOptions;
use sqlx::ConnectOptions; // Added import
use std::env;
use dotenv::dotenv;
use sqlx::Row; // Added for .get

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("🔌 Connecting to: {}", db_url.split('@').last().unwrap_or("???")); 

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

    println!("✅ Connected! Checking 'tasks' table columns...");

    // Use runtime query function to avoid macro expansion issues
    let rows = sqlx::query(
        "SELECT column_name, data_type 
         FROM information_schema.columns 
         WHERE table_name = 'tasks' 
         ORDER BY ordinal_position;"
    )
    .fetch_all(&pool)
    .await?;

    for row in rows {
        let name: String = row.get("column_name");
        let dtype: String = row.get("data_type");
        print!("- {}: {}", name, dtype);
        if name == "marketing_data" {
            print!("  <-- ✅ NEW COLUMN FOUND!");
        }
        println!();
    }

    Ok(())
}
