use rust_crawler::crawler;
use rust_crawler::stealth;
use rust_crawler::ml;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Starting Verification: Google Search with Cookies & Safety");
    
    // Define test query
    let keyword = "best crm software for small business";
    
    // 1. Run Search
    println!("🔎 Searching for: {}", keyword);
    let result = crawler::search_google(keyword).await;
    
    match result {
        Ok(data) => {
            println!("✅ Search SUCCESS!");
            println!("Found {} results", data.results.len());
            
            // 2. Inspect Marketing Data (if any result was deep-crawled/extracted)
            // Note: search_google returns SerpData. Deep extraction happens on individual pages.
            // For this test, let's also visit the first result to test deep extraction + marketing data.
            
            if let Some(first_result) = data.results.first() {
                println!("🌐 Visiting first result: {}", first_result.link);
                match crawler::extract_website_data(&first_result.link).await {
                    Ok(site_data) => {
                        println!("✅ Extraction SUCCESS!");
                        println!("Title: {}", site_data.title);
                        if let Some(marketing) = site_data.marketing_data {
                            println!("📢 Marketing Data Found:");
                            println!("   Headlines: {:?}", marketing.headlines);
                            println!("   Benefits: {:?}", marketing.key_benefits);
                            println!("   CTAs: {:?}", marketing.ctas);
                        } else {
                            println!("⚠️ No Marketing Data found (Selectors didn't match?)");
                        }
                    },
                    Err(e) => println!("❌ Extraction Failed: {}", e),
                }
            }
        },
        Err(e) => {
            println!("❌ Search FAILED: {}", e);
            println!("Check 'debug/debug_google_challenge.png' if it exists.");
        }
    }
    
    Ok(())
}
