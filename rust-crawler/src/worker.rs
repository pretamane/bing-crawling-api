use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::api::AppState;
use crate::crawler;
use crate::queue::CrawlJob;

pub async fn start_worker(state: Arc<AppState>) {
    println!("👷 Worker started, polling Redis...");

    loop {
        // Poll for 1 job
        match state.queue.pop_job().await {
            Ok(Some(job)) => {
                println!("👷 [Worker] Picked up job: {} ({})", job.id, job.keyword);
                if let Err(e) = process_job(state.clone(), job).await {
                    eprintln!("❌ [Worker] Job failed: {}", e);
                    // TODO: Implement DLQ or Retry here
                }
            },
            Ok(None) => {
                // Queue empty, sleep backoff
                sleep(Duration::from_millis(1000)).await;
            },
            Err(e) => {
                eprintln!("🔥 [Worker] Redis error: {}", e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn process_job(state: Arc<AppState>, job: CrawlJob) -> anyhow::Result<()> {
    println!("🚀 [Worker] Processing: {}", job.keyword);
    let pool = state.pool.clone();
    let engine_clone = job.engine.clone();

    // 1. Search (Google/Bing/Generic)
    let search_results = if job.engine == "google" {
        crawler::search_google(&job.keyword).await
    } else if job.engine == "generic" {
        crawler::generic_crawl(&job.keyword, job.selectors).await
    } else {
        crawler::search_bing(&job.keyword).await
    };

    let serp_data = match search_results {
        Ok(data) => data,
        Err(e) => {
             // Log failure to DB?
             return Err(e);
        }
    };

    // 2. Extract Content (Deep Crawl)
    let first_result_data: Option<crawler::WebsiteData> = if let Some(first_result) = serp_data.results.first() {
        println!("🔍 [Worker] Deep extracting: {}", first_result.link);
        crawler::extract_website_data(&first_result.link).await.ok()
    } else {
        None
    };

    let results_json = serde_json::to_string(&serp_data).unwrap_or_default();

    // 3. Save to MinIO (Raw HTML)
    // Example: Store first page HTML if exists
    if let Some(ref data) = first_result_data {
        if !data.html.is_empty() {
            let s3_key = format!("{}/{}.html", job.engine, job.id);
            if let Err(e) = state.storage.store_html(&s3_key, &data.html).await {
                eprintln!("⚠️ [Worker] MinIO upload failed: {}", e);
            } else {
                println!("💾 [Worker] HTML saved to MinIO: {}", s3_key);
            }
        }
    }

    // Prepare data for DB
    let (extracted_text, extracted_html, md, ma, mdate, emails, phones, links, images, sentiment, entities, category, marketing) = if let Some(data) = &first_result_data {
        
        // --- AI/ML ENRICHMENT (Running Locally) ---
        // We call the Python Sidecar on localhost:8000
        let entities = crate::ml::extract_entities_remote(&data.main_text).await;
        let category = crate::ml::classify_content_remote(&data.main_text).await;

        (
            data.main_text.clone(),
            data.html.clone(),
            data.meta_description.clone(),
            data.meta_author.clone(),
            data.meta_date.clone(),
            serde_json::to_value(&data.emails).unwrap_or_default(),
            serde_json::to_value(&data.phone_numbers).unwrap_or_default(),
            serde_json::to_value(&data.outbound_links).unwrap_or_default(),
            serde_json::to_value(&data.images).unwrap_or_default(),
            data.sentiment.clone(),
            serde_json::to_value(&entities).unwrap_or_default(), // New: Entities
            category, // New: Category
            serde_json::to_value(&data.marketing_data).unwrap_or_default(), // New: Marketing Data
        )
    } else {
        (
            String::new(), 
            String::new(), 
            None, 
            None, 
            None, 
            serde_json::json!([]), 
            serde_json::json!([]), 
            serde_json::json!([]), 
            serde_json::json!([]),
            None,
            serde_json::json!([]),
            Option::<String>::None,
            serde_json::json!({})
        )
    };

    // 4. Save to DB
    // 4. Save to DB with Workaround for Supabase
    let mut conn = pool.acquire().await?;
    // Workaround: generic deallocate to prevent "prepared statement already exists"
    let _ = sqlx::query("DEALLOCATE ALL").execute(&mut *conn).await;

    sqlx::query(
        r#"
        INSERT INTO tasks (
            id, keyword, engine, status, results_json, 
            extracted_text, first_page_html, meta_description, meta_author, meta_date,
            emails, phone_numbers, outbound_links, images, sentiment,
            entities, category, marketing_data
        ) 
        VALUES ($1, $2, $3, 'completed', $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#
    )
    .bind(&job.id)
    .bind(&job.keyword)
    .bind(&job.engine)
    .bind(&results_json)
    .bind(&extracted_text)
    .bind(&extracted_html)
    .bind(&md)
    .bind(&ma)
    .bind(&mdate)
    .bind(&emails)
    .bind(&phones)
    .bind(&links)
    .bind(&images)
    .bind(&sentiment)
    .bind(&entities)
    .bind(&category)
    .bind(&marketing)
    .execute(&mut *conn)
    .await?;

    println!("✅ [Worker] Job {} completed successfully!", job.id);

    // 5. Send Notification
    // We manually insert into DB because the worker doesn't have the API state/auth/endpoints handy, 
    // but sharing the DB pool is sufficient.
    let notification_id = uuid::Uuid::new_v4().to_string();
    let message = format!("Crawl finished for '{}'. Category: {:?}", job.keyword, category.as_deref().unwrap_or("Unknown"));
    
    // We skip the email sending part here for simplicity/speed (or we could duplicate the logic),
    // primarily ensuring the in-app notification exists for the test flow.
    let _ = sqlx::query(
        "INSERT INTO notifications (id, user_id, notification_type, subject, message) VALUES ($1, $2, 'system', 'Crawl Completed', $3)"
    )
    .bind(&notification_id)
    .bind(&job.user_id)
    .bind(&message)
    .execute(&pool) // using the pool clone
    .await;

    Ok(())
}
