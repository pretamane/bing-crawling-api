use sqlx::{postgres::PgPool, Row};
use anyhow::Result;

pub async fn init_db(pool: &PgPool) -> Result<()> {
    // 1. Create table if not exists (Base schema)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id VARCHAR PRIMARY KEY,
            keyword VARCHAR NOT NULL,
            engine VARCHAR NOT NULL DEFAULT 'bing',
            status VARCHAR NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            results_json TEXT,
            extracted_text TEXT,
            first_page_html TEXT,
            meta_description TEXT,
            meta_author TEXT,
            meta_date TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 2. Schema Evolution: Add new columns if they don't exist
    // We use a separate query for each column to handle potential partial migrations gracefully
    
    // Emails (JSONB)
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS emails JSONB;").execute(pool).await.ok();

    // Phone Numbers (JSONB)
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS phone_numbers JSONB;").execute(pool).await.ok();

    // Outbound Links (JSONB)
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS outbound_links JSONB;").execute(pool).await.ok();
        
    // Images (JSONB)
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS images JSONB;").execute(pool).await.ok();

    // Sentiment (TEXT)
    sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS sentiment TEXT;").execute(pool).await.ok();

    // ML Entities (JSONB)
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS entities JSONB;")
        .execute(pool)
        .await;

    // ML Category (TEXT)
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS category TEXT;")
        .execute(pool)
        .await;

    // Marketing Data (JSONB)
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN IF NOT EXISTS marketing_data JSONB;")
        .execute(pool)
        .await;

    Ok(())
}
