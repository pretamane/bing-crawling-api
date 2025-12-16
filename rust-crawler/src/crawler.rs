use headless_chrome::{Browser, LaunchOptions};
use anyhow::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;
use once_cell::sync::Lazy;
use regex::Regex;

// Import from new proxy module
use crate::proxy::{PROXY_MANAGER, generate_proxy_auth_extension};

static USER_AGENTS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:124.0) Gecko/20100101 Firefox/124.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:124.0) Gecko/20100101 Firefox/124.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/123.0.0.0 Safari/537.36",
    ]
});

// ============================================================================
// Enhanced Data Structures for Deep Extraction
// ============================================================================

/// Basic search result from SERP
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

/// Enhanced SERP data with additional extracted elements
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SerpData {
    /// Organic search results
    pub results: Vec<SearchResult>,
    /// "People Also Ask" questions (Google)
    pub people_also_ask: Vec<String>,
    /// Related searches at bottom of page
    pub related_searches: Vec<String>,
    /// Featured snippet if present
    pub featured_snippet: Option<FeaturedSnippet>,
    /// Total results count (if shown)
    pub total_results: Option<String>,
}

/// Featured snippet content
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeaturedSnippet {
    pub content: String,
    pub source_url: Option<String>,
    pub source_title: Option<String>,
}

/// Deep website data extraction
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WebsiteData {
    // Basic metadata
    pub url: String,
    pub final_url: String,
    pub title: String,
    pub meta_description: Option<String>,
    pub meta_keywords: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
    
    // Content extraction
    pub main_text: String,
    // HTML content (for saving to file)
    #[serde(skip)] 
    pub html: String,
    pub word_count: u32,
    pub html_size: u32,
    
    // Structured data (JSON-LD, Schema.org)
    pub schema_org: Vec<serde_json::Value>,
    
    // Open Graph data
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_type: Option<String>,
    
    // Contact information
    pub emails: Vec<String>,
    pub phone_numbers: Vec<String>,
    
    // Media
    pub images: Vec<ImageData>,
    
    // Links
    pub outbound_links: Vec<String>,
    
    // ML Analysis
    pub sentiment: Option<String>,
    
    // Marketing / Selling Points
    pub marketing_data: Option<MarketingData>,
}

/// Marketing and Selling Point Data
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MarketingData {
    /// H1/H2 Headlines (Value Props)
    pub headlines: Vec<String>,
    /// List items in feature sections (Benefits)
    pub key_benefits: Vec<String>,
    /// Button text (Calls to Action)
    pub ctas: Vec<String>,
}

/// Image data with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageData {
    pub src: String,
    pub alt: Option<String>,
    pub title: Option<String>,
}

/// Complete crawl result with all extracted data
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CrawlResult {
    pub keyword: String,
    pub engine: String,
    pub serp_data: SerpData,
    pub first_result_data: Option<WebsiteData>,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedContent {
    pub html: String,
    pub text: String,
    pub meta_description: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
}

// Cookie Struct for Injection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
}

// Map domain to list of cookies
pub type CookieMap = std::collections::HashMap<String, Vec<Cookie>>;

// ============================================================================
// Cookie Helper Functions
// ============================================================================

/// Load cookies from JSON file
pub fn load_cookies(domain_key: &str) -> Option<Vec<Cookie>> {
    let cookie_file = "cookies.json";
    if !std::path::Path::new(cookie_file).exists() {
        println!("🍪 No cookies.json found. Skipping cookie injection.");
        return None;
    }

    match std::fs::read_to_string(cookie_file) {
        Ok(content) => {
            match serde_json::from_str::<CookieMap>(&content) {
                Ok(map) => {
                    if let Some(cookies) = map.get(domain_key) {
                        println!("🍪 Found {} cookies for {}", cookies.len(), domain_key);
                        return Some(cookies.clone());
                    } else {
                        println!("🍪 No cookies found for domain: {}", domain_key);
                    }
                },
                Err(e) => println!("⚠️ Failed to parse cookies.json: {}", e),
            }
        },
        Err(e) => println!("⚠️ Failed to read cookies.json: {}", e),
    }
    None
}

/// Inject cookies into browser using CDP
pub fn inject_cookies(tab: &std::sync::Arc<headless_chrome::Tab>, cookies: &[Cookie]) -> Result<()> {
    use headless_chrome::protocol::cdp::Network;
    
    println!("🍪 Injecting {} cookies...", cookies.len());
    for cookie in cookies {
        // We use Network.setCookie for each cookie
        // Note: This is synchronous and might fail if domain doesn't match current context,
        // but typically works if done before navigation or on about:blank with domain specified.
        let result = tab.call_method(Network::SetCookie {
            name: cookie.name.clone(),
            value: cookie.value.clone(),
            url: None,
            domain: Some(cookie.domain.clone()),
            path: Some(cookie.path.clone()),
            secure: Some(cookie.secure),
            http_only: Some(false), // Optional
            same_site: None,
            expires: None,
            priority: None, 
            same_party: None,
            source_scheme: None,
            source_port: None,
            partition_key: None,
        });

        if let Err(e) = result {
             println!("⚠️ Failed to set cookie {}: {}", cookie.name, e);
        }
    }
    
    Ok(())
}

/// Random Sleep to simulate human specific behavior (High Latency)
/// Used for Account Safety to prevent rate limit flags.
pub async fn safe_sleep() {
    // Random float between 5.0 and 12.0 seconds
    let sleep_secs: f64 = {
        let mut rng = rand::thread_rng();
        use rand::Rng; 
        rng.gen_range(5.0..12.0)
    };
    
    println!("🛡️ Safety Sleep: Pausing for {:.1}s...", sleep_secs);
    sleep(Duration::from_millis((sleep_secs * 1000.0) as u64)).await;
}

/// Safe Human-Like Scrolling (Variable Speed/Length)
pub async fn scroll_safe(tab: &std::sync::Arc<headless_chrome::Tab>) -> Result<()> {
    println!("🛡️ Scrolling safely...");
    let script = r#"
        (async () => {
            const delay = ms => new Promise(res => setTimeout(res, ms));
            const totalHeight = document.body.scrollHeight;
            let currentHeight = 0;
            while(currentHeight < totalHeight) {
                 // Random scroll amount
                 const scrollStep = Math.floor(Math.random() * 400) + 100;
                 window.scrollBy(0, scrollStep);
                 currentHeight += scrollStep;
                 // Random pause
                 await delay(Math.floor(Math.random() * 1000) + 500); 
            }
        })()
    "#;
    tab.evaluate(script, true)?; // Await promise? No, evaluate is sync unless we poll.
    // Actually evaluate doesn't wait for async JS unless we use a wrapper.
    // For simplicity, we just sleep on Rust side while JS runs, or we implement a simpler scroll.
    
    // Simpler rust-side scroll loop
    for _ in 0..5 {
        let _ = tab.evaluate("window.scrollBy(0, window.innerHeight * 0.8);", false);
        safe_sleep().await;
    }
    Ok(())
}

/// Check if the current page is a known Ban/Checkpoint page
pub fn check_for_ban(tab: &std::sync::Arc<headless_chrome::Tab>) -> Result<()> {
    // Fast check via URL first
    let url = tab.get_url();
    if url.contains("checkpoint") || url.contains("challenge") || url.contains("suspicious") || url.contains("banned") {
        return Err(anyhow::anyhow!("🛑 CRITICAL: Checkpoint/Ban URL Detected: {}", url));
    }

    // Deep check content if URL is generic
    match tab.get_content() {
        Ok(html) => {
            if html.contains("Verify it's you") || html.contains("security check") || html.contains("temporarily locked") {
                 return Err(anyhow::anyhow!("🛑 CRITICAL: Checkpoint Content Detected"));
            }
        },
        Err(_) => {} // Ignore content check failure
    }
    
    Ok(())
}

// ============================================================================
// Extraction Helper Functions
// ============================================================================

/// Extract emails from text using regex
pub fn extract_emails(text: &str) -> Vec<String> {
    let email_regex = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    email_regex
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Extract phone numbers from text using regex
pub fn extract_phone_numbers(text: &str) -> Vec<String> {
    let phone_regex = Regex::new(r"[\+]?[(]?[0-9]{1,3}[)]?[-\s\.]?[(]?[0-9]{1,4}[)]?[-\s\.]?[0-9]{1,4}[-\s\.]?[0-9]{1,9}").unwrap();
    phone_regex
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .filter(|p| p.len() >= 7) // Filter out short matches
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Extract Schema.org JSON-LD data from HTML
pub fn extract_schema_org(html: &str) -> Vec<serde_json::Value> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("script[type='application/ld+json']").unwrap();
    
    document
        .select(&selector)
        .filter_map(|el| {
            let json_text = el.text().collect::<String>();
            serde_json::from_str(&json_text).ok()
        })
        .collect()
}

/// Extract Open Graph metadata
pub fn extract_open_graph(document: &Html) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let og_title = document
        .select(&Selector::parse("meta[property='og:title']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    let og_description = document
        .select(&Selector::parse("meta[property='og:description']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    let og_image = document
        .select(&Selector::parse("meta[property='og:image']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    let og_type = document
        .select(&Selector::parse("meta[property='og:type']").unwrap())
        .next()
        .and_then(|el| el.value().attr("content").map(|s| s.to_string()));
    
    (og_title, og_description, og_image, og_type)
}

/// Extract images with metadata
pub fn extract_images(document: &Html, base_url: &str) -> Vec<ImageData> {
    let img_selector = Selector::parse("img").unwrap();
    
    document
        .select(&img_selector)
        .filter_map(|el| {
            let src = el.value().attr("src").or_else(|| el.value().attr("data-src"))?;
            // Skip tiny/tracking pixels
            if src.contains("1x1") || src.contains("pixel") || src.len() < 10 {
                return None;
            }
            Some(ImageData {
                src: if src.starts_with("http") { src.to_string() } else { format!("{}{}", base_url, src) },
                alt: el.value().attr("alt").map(|s| s.to_string()),
                title: el.value().attr("title").map(|s| s.to_string()),
            })
        })
        .take(20) // Limit to first 20 images
        .collect()
}

/// Extract outbound links
pub fn extract_outbound_links(document: &Html, base_domain: &str) -> Vec<String> {
    let link_selector = Selector::parse("a[href]").unwrap();
    
    document
        .select(&link_selector)
        .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
        .filter(|href| href.starts_with("http") && !href.contains(base_domain))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(50) // Limit to 50 links
        .collect()
}


// Wrapper with Retry Logic for Bing
pub async fn search_bing(keyword: &str) -> Result<SerpData> {
    println!("🔎 Starting Bing Deep Search for: {}", keyword);
    let mut last_error = String::from("No results found");
    
    // Max 3 attempts
    for attempt in 1..=3 {
        if attempt > 1 { println!("🔄 Retry Attempt {}/3...", attempt); }

        match search_bing_attempt(keyword).await {
            Ok(data) => {
                if data.results.is_empty() {
                    println!("⚠️ Attempt {}/3: Bing returned 0 results.", attempt);
                    if attempt < 3 {
                        let wait_time = 5 * attempt as u64;
                        println!("⏳ Waiting {}s before retry...", wait_time);
                        sleep(Duration::from_secs(wait_time)).await;
                        continue;
                    }
                } else {
                    println!("✅ Attempt {}/3: Success! Found {} results.", attempt, data.results.len());
                    return Ok(data);
                }
            }
            Err(e) => {
                println!("❌ Attempt {}/3: Error: {}", attempt, e);
                last_error = e.to_string();
                if attempt < 3 { sleep(Duration::from_secs(5)).await; }
            }
        }
    }
    Err(anyhow::anyhow!("Bing search failed after 3 attempts. Last error: {}", last_error))
}

// Internal attempt function for Bing
async fn search_bing_attempt(keyword: &str) -> Result<SerpData> {
    use rand::seq::SliceRandom;
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/123.0.0.0 Safari/537.36");
    
    // Use anonymous/incognito mode
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--incognito"),
        std::ffi::OsStr::new("--headless=new"),
    ];
    let ua_arg = format!("--user-agent={}", user_agent);
    args.push(std::ffi::OsStr::new(&ua_arg));

    // Proxy config (same as Google)
    let current_proxy = PROXY_MANAGER.get_next_proxy();
    // Keep string alive for args
    let mut proxy_arg = String::new(); 
    
    if let Some(ref proxy) = current_proxy {
        proxy_arg = format!("--proxy-server={}", proxy.to_chrome_arg());
        args.push(std::ffi::OsStr::new(&proxy_arg));
        // Auth extension logic omitted for brevity in this block but should ideally be shared
    } else {
        println!("📡 No proxies configured. Using direct connection.");
    }

    let browser = Browser::new(LaunchOptions {
        headless: false, 
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    
    // Inject Stealth
    let stealth_script = crate::stealth::get_stealth_script();
    tab.enable_debugger()?;
    tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_script.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    // Apply Fingerprint Overrides (Timezone/Locale) matching IP
    if let Err(e) = crate::stealth::apply_stealth_settings(&tab, "Asia/Yangon", "en-US").await {
         eprintln!("Failed to apply stealth settings: {}", e);
    }

    // 1. Navigate to Home (Force US Market)
    println!("Navigating to Bing Home...");
    tab.navigate_to("https://www.bing.com/?setmkt=en-US&setlang=en-us")?;
    tab.wait_until_navigated()?;
    
    sleep(Duration::from_millis(2000 + (rand::random::<u64>() % 2000))).await;

    // Handle Consent (Universal ID check)
    println!("Checking for consent page...");
    tab.evaluate(r#"
        (() => {
            const selectors = ['button[id="bnp_btn_accept"]', 'button[id="onetrust-accept-btn-handler"]'];
            for (const sel of selectors) {
                const btn = document.querySelector(sel);
                if (btn) { btn.click(); console.log("Clicked consent: " + sel); }
            }
        })();
    "#, false)?;

    // 2. Type Query
    println!("Waiting for search box...");
    let search_box = tab.wait_for_element("textarea[name='q'], input[name='q'], #sb_form_q")?;
    
    println!("Clicking search box...");
    tab.evaluate(r#"
        const input = document.querySelector("textarea[name='q'], input[name='q'], #sb_form_q");
        if (input) { input.click(); input.focus(); input.value = ''; }
    "#, false)?;
    sleep(Duration::from_millis(500)).await;

    println!("Typing query: {}...", keyword);
    for char in keyword.chars() {
        tab.type_str(&char.to_string())?;
        sleep(Duration::from_millis(80 + (rand::random::<u64>() % 100))).await;
    }
    sleep(Duration::from_millis(500)).await;

    // 3. Submit
    println!("Submitting search...");
    tab.press_key("Enter")?;
    tab.wait_until_navigated()?;
    println!("Search submitted.");

    // Check for Challenge AFTER search
    sleep(Duration::from_secs(3)).await;
    let html_content = tab.get_content()?;
    if html_content.contains("Challenge") || html_content.contains("needs to review the security") {
         println!("⚠️ CHALLENGE DETECTED: Bing served Challenge/Captcha page");
         let _ = tab.capture_screenshot(headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png, None, None, true)
            .map(|s| std::fs::write("debug/debug_bing_challenge.png", s));
         return Err(anyhow::anyhow!("Bing Challenge Detected"));
    }

    // Extract Data
    println!("Extraction method: dom");
    let document = Html::parse_document(&html_content);
    let mut results = Vec::new();
    
    // Bing Organic Selector: #b_results > li.b_algo
    let result_selector = Selector::parse("#b_results > li.b_algo").unwrap();
    for element in document.select(&result_selector) {
        let title_sel = Selector::parse("h2 a").unwrap();
        let snippet_sel = Selector::parse(".b_caption p").unwrap();
        
        let title = element.select(&title_sel).next().map(|e| e.text().collect::<String>()).unwrap_or_default();
        let link = element.select(&title_sel).next().and_then(|e| e.value().attr("href")).unwrap_or_default().to_string();
        let snippet = element.select(&snippet_sel).next().map(|e| e.text().collect::<String>()).unwrap_or_default();
        
        if !title.is_empty() && !link.is_empty() {
             results.push(SearchResult { title, link, snippet });
        }
    }

    Ok(SerpData {
         results,
         related_searches: vec![],
         people_also_ask: vec![],
         total_results: None,
         featured_snippet: None
    })
}

pub async fn search_google(keyword: &str) -> Result<SerpData> {
    println!("🔎 Starting Google Deep Search for: {}", keyword);
    let mut last_error = String::from("No results found");
    
    // Max 3 attempts for resilience
    for attempt in 1..=3 {
        if attempt > 1 {
             println!("🔄 Retry Attempt {}/3...", attempt);
        }

        match search_google_attempt(keyword, attempt).await {
            Ok(data) => {
                if data.results.is_empty() {
                    println!("⚠️ Attempt {}/3: Google returned 0 results (Block/Captcha?).", attempt);
                    if attempt < 3 {
                        let wait_time = 5 * attempt as u64;
                        println!("⏳ Waiting {}s before retry...", wait_time);
                        sleep(Duration::from_secs(wait_time)).await;
                        continue;
                    }
                } else {
                    println!("✅ Attempt {}/3: Success! Found {} results.", attempt, data.results.len());
                    return Ok(data);
                }
            }
            Err(e) => {
                println!("❌ Attempt {}/3: Error: {}", attempt, e);
                last_error = e.to_string();
                if attempt < 3 {
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Google search failed after 3 attempts. Last error: {}", last_error))
}

// Internal attempt function
async fn search_google_attempt(keyword: &str, attempt: u32) -> Result<SerpData> {
    use rand::seq::SliceRandom;
    let user_agent = if attempt == 3 {
        // Mobile Agents for Attempt 3
        static MOBILE_AGENTS: &[&str] = &[
            "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Mobile Safari/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Mobile/15E148 Safari/604.1",
        ];
        MOBILE_AGENTS.choose(&mut rand::thread_rng()).unwrap()
    } else {
        USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
    };
    
    println!("Using User-Agent (Attempt {}): {}", attempt, user_agent);

    // Use anonymous/incognito mode (no profile persistence)
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
        std::ffi::OsStr::new("--incognito"),
    ];
    let ua_arg = format!("--user-agent={}", user_agent);
    args.push(std::ffi::OsStr::new(&ua_arg));

    // Use modern headless mode
    args.push(std::ffi::OsStr::new("--headless=new"));

    // Add proxy if available (using new ProxyManager)
    let proxy_arg: String;
    let ext_arg: String;
    let current_proxy = PROXY_MANAGER.get_next_proxy();
    let _proxy_id = current_proxy.as_ref().map(|p| p.id.clone());
    
    if let Some(ref proxy) = current_proxy {
        println!("🔄 Using proxy: {} (healthy: {}, success_rate: {:.1}%)", 
            proxy.id, 
            proxy.healthy.load(std::sync::atomic::Ordering::Relaxed),
            proxy.success_rate() * 100.0
        );
        proxy_arg = format!("--proxy-server={}", proxy.to_chrome_arg());
        args.push(std::ffi::OsStr::new(&proxy_arg));
        
        // Add auth extension if proxy requires authentication
        if proxy.requires_auth() {
            let ext_path = generate_proxy_auth_extension(
                proxy.username.as_ref().unwrap(),
                proxy.password.as_ref().unwrap()
            );
            ext_arg = format!("--load-extension={}", ext_path);
            args.push(std::ffi::OsStr::new(&ext_arg));
            println!("🔐 Proxy auth extension loaded");
        }
    }

    let browser = Browser::new(LaunchOptions {
        headless: false, // Use new headless mode via args
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;

    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    // Layer 1: Device & Environment Fingerprinting (JS-Level)
    let stealth_script = crate::stealth::get_stealth_script();

    tab.enable_debugger()?;
    tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_script.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    // Apply Fingerprint Overrides (Timezone/Locale) for Residential IP
    if let Err(e) = crate::stealth::apply_stealth_settings(&tab, "Asia/Yangon", "en-US").await {
         eprintln!("Failed to apply stealth settings: {}", e);
    }

    // URL Construction Strategy
    let mut url = "https://www.google.com/?hl=en".to_string();
    // Attempt 1: Force US (previous default). Attempts 2+: Local/No GL (avoid geo mismatch).
    if attempt == 1 {
        url.push_str("&gl=us");
    }
    
    // Inject cookies for Google
    if let Some(cookies) = load_cookies("google.com") {
        let _ = inject_cookies(&tab, &cookies);
    }
    
    println!("Navigating to Google Home (Attempt {}, URL: {})...", attempt, url);
    tab.navigate_to(&url)?;
    tab.wait_until_navigated()?;
    
    // Random wait to simulate reading
    sleep(Duration::from_millis(3000 + (rand::random::<u64>() % 2000))).await;

    // Handle consent page (if present)
    println!("Checking for consent page...");
    let consent_result = tab.evaluate(r#"
        (() => {
            // Universal / Language Agnostic Consent Handlers
            // We check for the BUTTON itself, not the text.
            const selectors = [
                'button[id="L2AGLb"]', // ID: Accept all (Global)
                'button[id="W0wltc"]', // ID: Reject all (Global)
                'button[id*="agree"]', // ID heuristic
                'button[id*="accept"]', // ID heuristic
                'form[action*="consent"] button', // Form heuristic
                'div[role="dialog"] button:last-of-type' // Structure: Last button in modal
            ];

            for (const selector of selectors) {
                 const btn = document.querySelector(selector);
                 if (btn && btn.offsetParent !== null) { // Ensure visible
                     console.log("Found consent button: " + selector);
                     btn.click();
                     return "consent_clicked";
                 }
            }
            return "no_consent";
        })();
    "#, false)?;
    
    if let Some(serde_json::Value::String(result)) = consent_result.value {
        println!("Consent check result: {}", result);
        if result == "consent_clicked" {
            println!("Consent accepted, waiting for redirect...");
            sleep(Duration::from_secs(2)).await;
            tab.wait_until_navigated()?;
        }
    }
    
    // Human-like mouse movement (entropy)
    // Native Human Mouse Movement (CDP-based)
    println!("Simulating native human mouse movements...");
    // Move towards center 
    let start = crate::stealth::Point::new(100.0, 100.0);
    // Approx center
    let end = crate::stealth::Point::new(500.0, 300.0); 
    if let Err(e) = crate::stealth::move_mouse_human(&tab, start, end).await {
         println!("Native mouse move failed: {}", e);
    }

    sleep(Duration::from_millis(1000)).await;
    
    // Take screenshot for debugging
    println!("Capturing screenshot for debugging...");
    if let Ok(screenshot) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None,
        None,
        true
    ) {
        let _ = std::fs::write("debug/debug_google_screenshot.png", &screenshot);
        println!("Screenshot saved to debug/debug_google_screenshot.png");
    }

    // 2. Type Query (Layer 3: Typing Speed)
    // Google uses textarea[name='q'] or input[name='q'] depending on version/AB test.
    // Try multiple selectors with retries
    println!("Waiting for search box...");
    let selectors = ["textarea[name='q']", "input[name='q']", "textarea[title*='Search']", "input[title*='Search']"];
    let mut search_box_result = None;
    
    for selector in selectors {
        println!("Trying selector: {}", selector);
        match tab.wait_for_element_with_custom_timeout(selector, std::time::Duration::from_secs(10)) {
            Ok(el) => {
                println!("✅ Found search box with: {}", selector);
                search_box_result = Some(el);
                break;
            },
            Err(e) => {
                println!("⚠️ Selector '{}' failed: {}", selector, e);
            }
        }
    }
    
    let search_box = search_box_result.ok_or_else(|| anyhow::anyhow!("No search box selector worked"))?;
    
    // Wait for React/JS to finish rendering
    println!("Waiting for search box to become interactive...");
    sleep(Duration::from_millis(1000)).await;
    
    // Use JS to click and focus (more reliable than CDP click for dynamic elements)
    println!("Clicking and focusing search box via JS...");
    tab.evaluate(r#"
        const input = document.querySelector('textarea[name="q"]') || document.querySelector('input[name="q"]');
        if (input) { 
            input.click(); 
            input.focus(); 
            input.value = ''; 
        }
    "#, false)?;
    sleep(Duration::from_millis(500)).await;
    
    // Type query naturally for personalized results (profile-based)
    println!("Typing query: {}...", keyword);
    for char in keyword.chars() {
        tab.type_str(&char.to_string())?;
        sleep(Duration::from_millis(100 + (rand::random::<u64>() % 150))).await;
    }
    
    sleep(Duration::from_millis(500)).await;

    // 3. Submit
    println!("Submitting search...");
    tab.press_key("Enter")?;
    tab.wait_until_navigated()?;
    println!("Search submitted.");

    // Check for Challenge/Captcha immediately after navigation
    sleep(Duration::from_secs(2)).await;
    let html_content = tab.get_content()?;
    if html_content.contains("unusual traffic") || html_content.contains("captcha-form") || html_content.contains("systems have detected") {
         println!("⚠️ CHALLENGE DETECTED: Google served Captcha/Unusual Traffic page");
         let _ = tab.capture_screenshot(headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png, None, None, true)
            .map(|s| std::fs::write("debug/debug_google_challenge.png", s));
         return Err(anyhow::anyhow!("Google Challenge Detected"));
    }
    
    // Check for Google autocorrection message and click "Search instead for [exact term]"
    // Wait longer for the "Search instead for" link to appear
    sleep(Duration::from_millis(3000)).await;
    let verbatim_result = tab.evaluate(r#"
        (() => {
            // Helper to find link by text
            const findLinkByText = (text) => {
                const links = document.querySelectorAll('a');
                for (const link of links) {
                    if (link.textContent.includes(text)) return link;
                }
                return null;
            };

            // 1. Look for "Search instead for" link
            const verbatimLink = document.querySelector('a.spell_orig') || 
                                  document.querySelector('a[href*="nfpr=1"]') ||
                                  document.querySelector('#fprsl') ||
                                  findLinkByText("Search instead for");
            
            if (verbatimLink) {
                console.log('[VERBATIM] Found original search link, clicking...');
                verbatimLink.click();
                return "clicked_verbatim";
            }

            // 2. Check for "Showing results for" (standard autocorrect)
            const showingFor = document.querySelector('.spell') || document.querySelector('#scl');
            if (showingFor) {
                const originalLink = showingFor.querySelector('a');
                if (originalLink) {
                    originalLink.click();
                    return "clicked_original";
                }
            }
            return "no_autocorrect";
        })();
    "#, false)?;
    
    if let Some(serde_json::Value::String(result)) = verbatim_result.value {
        println!("Verbatim check result: {}", result);
        if result != "no_autocorrect" {
            println!("Clicked verbatim link, waiting for reload...");
            sleep(Duration::from_secs(2)).await;
            tab.wait_until_navigated()?;
        }
    }

    // Layer 3: Behavioral Realism
    // Native Human Mouse Movement (Behavioral)
    let start = crate::stealth::Point::new(100.0, 100.0);
    let end = crate::stealth::Point::new(500.0, 400.0);
    if let Err(e) = crate::stealth::move_mouse_human(&tab, start, end).await {
         println!("Native mouse move failed: {}", e);
    }
    
    sleep(Duration::from_millis(500)).await;

    // Native Human Scroll
    if let Err(e) = crate::stealth::scroll_human(&tab, 800.0).await {
        println!("Native scroll failed: {}", e);
    }

    // L3: Google Extraction Strategy (CDP-Based, Per Debug Sequence)
    // Step 1: ✅ Already navigating to homepage → typing → submit (not direct SERP URL)
    
    // Add static wait for Google JS to initialize before mutation observer
    println!("Waiting 3s for Google JS to initialize...");
    sleep(Duration::from_secs(3)).await;
    
    // Step 2: Mutation observer with increased timeout (15s) and logging
    println!("Waiting for Google DOM mutations to complete...");
    let wait_script = r#"
        new Promise((resolve) => {
            let timeout;
            let mutationCount = 0;
            const observer = new MutationObserver(() => {
                mutationCount++;
                console.log(`[MUTATION] Count: ${mutationCount}`);
                clearTimeout(timeout);
                timeout = setTimeout(() => {
                    console.log(`[MUTATION] Settled after ${mutationCount} mutations`);
                    observer.disconnect();
                    resolve("mutations_complete");
                }, 1000); // Increased debounce: 500ms → 1000ms
            });
            observer.observe(document.body, { childList: true, subtree: true });
            
            // Increased fallback timeout: 5s → 12s
            setTimeout(() => {
                console.log(`[MUTATION] Timeout reached after ${mutationCount} mutations`);
                observer.disconnect();
                resolve("timeout_reached");
            }, 12000);
        });
    "#;
    
    let wait_result = tab.evaluate(wait_script, true)?;
    println!("DOM wait result: {:?}", wait_result.value);
    
    // Step 3: Extract via semantic attributes (resilient to class changes)
    let extraction_method: String;
    let results: Vec<SearchResult>;
    
    // Method 1: DOM extraction using expanded selectors (Step 5)
    let dom_extract_script = r#"
        (() => {
            const results = [];
            const mainContent = document.querySelector('[role="main"]') || document.querySelector('#main');
            
            if (!mainContent) {
                console.log('[EXTRACT] No main content found');
                return JSON.stringify({method: "dom", results: [], error: "no_main"});
            }
            
            console.log('[EXTRACT] Main content found');
            
            // Step 5: Expanded selectors (union of known Google containers)
            const resultBlocks = mainContent.querySelectorAll(
                '[data-snf], .g, [jscontroller="SC7lYd"], [data-ved], .Gx5Zad'
            );
            
            console.log(`[EXTRACT] Found ${resultBlocks.length} result blocks`);
            
            // Step 4: DOM Snapshot Fallback
            if (resultBlocks.length === 0 && !document.querySelector('[role="main"] h3')) {
                console.log('[EXTRACT] No blocks found, trying script tag fallback');
                const scriptData = Array.from(document.scripts).find(s => 
                    s.textContent?.includes('"results":') || s.textContent?.includes('AF_initDataCallback')
                );
                if (scriptData) {
                    return JSON.stringify({
                        method: "script_fallback", 
                        results: [], 
                        raw_snippet: scriptData.textContent.substring(0, 200)
                    });
                }
            }
            
            resultBlocks.forEach((block, idx) => {
                const titleEl = block.querySelector('h3, [role="heading"]');
                const linkEl = block.querySelector('a[href^="http"]:not([href*="google.com"])') || 
                              block.querySelector('a[jsname]');
                const snippetEl = block.querySelector('[data-content], [role="text"], .VwiC3b, .IsZvec, .yXK7lf');
                
                if (titleEl && linkEl && linkEl.href && !linkEl.href.includes('google.com/search')) {
                    console.log(`[EXTRACT] Block ${idx}: ${titleEl.textContent.trim().substring(0, 30)}`);
                    results.push({
                        title: titleEl.textContent.trim(),
                        link: linkEl.href,
                        snippet: snippetEl ? snippetEl.textContent.trim() : ""
                    });
                }
            });
            
            console.log(`[EXTRACT] Returning ${results.length} results`);
            return JSON.stringify({method: "dom", results: results.slice(0, 10)});
        })();
    "#;
    
    match tab.evaluate(dom_extract_script, true) {
        Ok(result) => {
            if let Some(serde_json::Value::String(value_str)) = result.value {
                let parsed: serde_json::Value = serde_json::from_str(&value_str).unwrap_or_default();
                extraction_method = parsed["method"].as_str().unwrap_or("unknown").to_string();
                results = serde_json::from_value(parsed["results"].clone()).unwrap_or_default();
                println!("Extracted {} results via method: {}", results.len(), extraction_method);
            } else {
                extraction_method = "fallback".to_string();
                results = Vec::new();
            }
        }
        Err(e) => {
            eprintln!("DOM extraction failed: {}, trying JS context fallback", e);
            extraction_method = "js_context".to_string();
            
            // Method 2: JS Context fallback (window.google.search.cse)
            let js_extract_script = r#"
                (() => {
                    try {
                        const googleData = window.google?.search?.cse?.results?.[0]?.results || [];
                        return JSON.stringify({
                            method: "js_context",
                            results: googleData.slice(0, 10).map(r => ({
                                title: r.title || "",
                                link: r.url || "",
                                snippet: r.content || ""
                            }))
                        });
                    } catch(e) {
                        return JSON.stringify({method: "js_context", results: []});
                    }
                })();
            "#;
            
            match tab.evaluate(js_extract_script, true) {
                Ok(js_result) => {
                    if let Some(serde_json::Value::String(value_str)) = js_result.value {
                        let parsed: serde_json::Value = serde_json::from_str(&value_str).unwrap_or_default();
                        results = serde_json::from_value(parsed["results"].clone()).unwrap_or_default();
                    } else {
                        results = Vec::new();
                    }
                }
                Err(_) => {
                    results = Vec::new();
                }
            }
        }
    }
    
    println!("Extraction method: {}", extraction_method);
    
    println!("Found {} results.", results.len());

    if results.is_empty() {
        let html_content = tab.get_content().unwrap_or_default();
        eprintln!("Google returned 0 results. HTML len: {}", html_content.len());
        let _ = std::fs::write("debug/debug_google_tier1.html", &html_content);
    }

    // Extract People Also Ask
    let html_content = tab.get_content()?;
    let document = Html::parse_document(&html_content);
    
    let paa_selector = Selector::parse(".related-question-pair .s75CSd").unwrap();
    let mut people_also_ask: Vec<String> = Vec::new(); // Explicit type
    for element in document.select(&paa_selector) {
        if let Some(text) = element.text().next() {
            people_also_ask.push(text.to_string());
        }
    }

    // Extract Related Searches
    let related_selector = Selector::parse(".s75CSd, .k8XOCe, .related-searches-list a").unwrap();
    let mut related_searches: Vec<String> = Vec::new(); // Explicit type
    for element in document.select(&related_selector) {
         if let Some(text) = element.text().next() {
             let s = text.to_string();
             if s.len() > 3 {
                 related_searches.push(s);
             }
         }
    }

    // Extract Total Results
    let count_selector = Selector::parse("#result-stats").unwrap();
    let total_results = document.select(&count_selector).next()
        .map(|e| e.text().collect::<String>());
        
    // Extract Featured Snippet
    let snippet_selector = Selector::parse(".xpdopen .block-component, .c2xzTb").unwrap();
    let featured_snippet: Option<FeaturedSnippet> = document.select(&snippet_selector).next().map(|el| {
        FeaturedSnippet {
            content: el.text().collect::<String>(),
            source_url: None,
            source_title: None,
        }
    });

    Ok(SerpData {
        results,
        people_also_ask,
        related_searches,
        featured_snippet,
        total_results,
    })
}

pub async fn extract_content(url: &str) -> Result<ExtractedContent> {
    // Decode Bing/Google redirect URLs to get actual destination
    let actual_url = decode_search_url(url);
    println!("Extracting content from: {}", actual_url);
    
    // Use proper User-Agent and follow redirects
    use rand::seq::SliceRandom;
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    let client = reqwest::Client::builder()
        .user_agent(*user_agent)
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let resp: reqwest::Response = client.get(&actual_url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send().await?;
    let final_url = resp.url().to_string();
    println!("Final URL after redirects: {}", final_url);
    
    let html = resp.text().await?;
    println!("Fetched HTML size: {} bytes", html.len());
    
    let mut reader = Cursor::new(html.as_bytes());
    
    // 1. Extract text with Readability
    let text = match readability::extractor::extract(&mut reader, &reqwest::Url::parse(&final_url)?) {
        Ok(product) => product.text,
        Err(_) => "Failed to extract content".to_string(),
    };

    // 2. Extract metadata manually using Scraper
    let document = Html::parse_document(&html);
    let desc_selector = Selector::parse("meta[name='description']").unwrap();
    let author_selector = Selector::parse("meta[name='author']").unwrap();
    let date_selector = Selector::parse("meta[property='article:published_time']").unwrap();

    let meta_description = document.select(&desc_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    
    let meta_author = document.select(&author_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));

    let meta_date = document.select(&date_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));

    Ok(ExtractedContent {
        html: html.clone(),
        text,
        meta_description,
        meta_author,
        meta_date,
    })
}

/// Deep extraction function that returns comprehensive WebsiteData using Headless Chrome
pub async fn extract_website_data(url: &str) -> Result<WebsiteData> {
    // Decode Bing/Google redirect URLs to get actual destination
    let actual_url = decode_search_url(url);
    println!("🔍 Deep integration extracting data from: {}", actual_url);
    
    use rand::seq::SliceRandom;
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    // Configure Chrome arguments for Stealth
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
    ];
    let ua_arg = format!("--user-agent={}", user_agent);
    args.push(std::ffi::OsStr::new(&ua_arg));

    // Use modern headless mode
    args.push(std::ffi::OsStr::new("--headless=new"));

    // Add proxy if available
    let current_proxy = PROXY_MANAGER.get_next_proxy();
    let proxy_arg: String;
    let ext_arg: String;
    
    if let Some(ref proxy) = current_proxy {
        proxy_arg = format!("--proxy-server={}", proxy.to_chrome_arg());
        args.push(std::ffi::OsStr::new(&proxy_arg));
        
        if proxy.requires_auth() {
            let ext_path = generate_proxy_auth_extension(
                proxy.username.as_ref().unwrap(),
                proxy.password.as_ref().unwrap()
            );
            ext_arg = format!("--load-extension={}", ext_path);
            args.push(std::ffi::OsStr::new(&ext_arg));
        }
    }

    // Launch Browser
    let browser = Browser::new(LaunchOptions {
        headless: false, // Use new headless mode via args
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;

    // Inject Stealth Script
    // Inject Stealth Script
    let stealth_script = crate::stealth::get_stealth_script();

    tab.enable_debugger()?;
    tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
        source: stealth_script.to_string(),
        world_name: None,
        include_command_line_api: None,
        run_immediately: None,
    })?;

    // Navigate
    println!("Navigating to: {}", actual_url);
    tab.navigate_to(&actual_url)?;
    
    // Use softer wait (wait for body) instead of strict load event to prevent timeouts on ads/tracking
    match tab.wait_for_element_with_custom_timeout("body", Duration::from_secs(15)) {
        Ok(_) => println!("Page body loaded."),
        Err(e) => println!("⚠️ Warning: Body wait timed out: {}. Attempting extraction anyway...", e),
    }

    // Wait for JS execution (Hydration)
    sleep(Duration::from_secs(4)).await;

    // Extract Data via JS
    let html = tab.evaluate("document.documentElement.outerHTML", false)?.value.unwrap().as_str().unwrap().to_string();
    let final_url = tab.get_url();
    let html_size = html.len() as u32;
    println!("Extracted HTML size via Browser: {} bytes", html_size);

    // 10. Marketing Data Extraction (Async - must be done before parsing document)
    let marketing_data = match extract_marketing_data(&tab).await {
        Ok(data) => Some(data),
        Err(e) => {
            println!("⚠️ Marketing extraction failed: {}", e);
            None
        }
    };

    // Parse document using Scraper for consistency with previous logic
    let document = Html::parse_document(&html);
    
    // Extract base domain
    let base_domain = reqwest::Url::parse(&final_url)
        .map(|u| u.host_str().unwrap_or("").to_string())
        .unwrap_or_default();
    
    // 1. Extract title
    let title = tab.evaluate("document.title", false)?.value.unwrap().as_str().unwrap().to_string();
    
    // 2. Extract meta tags using Scraper
    let desc_selector = Selector::parse("meta[name='description']").unwrap();
    let keywords_selector = Selector::parse("meta[name='keywords']").unwrap();
    let author_selector = Selector::parse("meta[name='author']").unwrap();
    let date_selector = Selector::parse("meta[property='article:published_time']").unwrap();
    
    let meta_description = document.select(&desc_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    let meta_keywords = document.select(&keywords_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    let meta_author = document.select(&author_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    let meta_date = document.select(&date_selector).next()
        .and_then(|e| e.value().attr("content").map(|s| s.to_string()));
    
    // 3. Extract main text using Readability on the rendered HTML
    let mut reader = Cursor::new(html.as_bytes());
    let main_text = match readability::extractor::extract(&mut reader, &reqwest::Url::parse(&final_url)?) {
        Ok(product) => product.text,
        Err(_) => {
            // Fallback to body text if Readability fails
            tab.evaluate("document.body.innerText", false)
                .map(|v| v.value.unwrap().as_str().unwrap().to_string())
                .unwrap_or_default()
        },
    };
    let word_count = main_text.split_whitespace().count() as u32;
    
    // 4. Extract Schema.org/JSON-LD structured data
    let schema_org = extract_schema_org(&html);
    if !schema_org.is_empty() {
        println!("📊 Found {} Schema.org objects", schema_org.len());
    }
    
    // 5. Extract Open Graph data
    let (og_title, og_description, og_image, og_type) = extract_open_graph(&document);
    
    // 6. Extract contact information
    let emails = extract_emails(&html);
    let phone_numbers = extract_phone_numbers(&main_text);
    
    // 7. Extract images
    let images = extract_images(&document, &format!("https://{}", base_domain));
    
    // 8. Extract outbound links
    let outbound_links = extract_outbound_links(&document, &base_domain);
    
    // 9. ML Sentiment Analysis
    let sentiment = crate::ml::analyze_sentiment(&main_text);
    if let Some(ref s) = sentiment {
        println!("🧠 Sentiment Analysis Result: {}", s);
    }

    Ok(WebsiteData {
        url: actual_url,
        final_url,
        title,
        meta_description,
        meta_keywords,
        meta_author,
        meta_date,
        main_text,
        html: html.clone(),
        word_count,
        html_size,
        schema_org,
        og_title,
        og_description,
        og_image,
        og_type,
        emails,
        phone_numbers,
        images,
        outbound_links,
        sentiment,
        marketing_data,
    })
}

/// Extract Marketing Data (Selling Points)
pub async fn extract_marketing_data(tab: &std::sync::Arc<headless_chrome::Tab>) -> Result<MarketingData> {
    println!("📢 Extracting Marketing Data (Selling Points)...");
    
    let script = r#"
        (() => {
            // 1. Headlines (Value Propositions)
            const headlines = Array.from(document.querySelectorAll('h1, h2'))
                .map(el => el.textContent.trim())
                .filter(t => t.length > 10 && t.length < 100) // Filter noise
                .slice(0, 5); // Top 5

            // 2. Key Benefits (List items in feature sections)
            // Heuristic: ul/li inside sections/divs with class 'feature', 'benefit', 'service'
            const benefitSelectors = [
                '[class*="feature"] li', '[class*="benefit"] li', 
                '.features li', '.benefits li', 
                '#features li', '#benefits li',
                'ul li' // Fallback: all list items (filtered below)
            ];
            
            let benefits = [];
            for (const sel of benefitSelectors) {
                const items = document.querySelectorAll(sel);
                if (items.length > 0) {
                     benefits = Array.from(items)
                        .map(el => el.textContent.trim())
                        .filter(t => t.length > 20 && t.length < 150);
                     if (benefits.length > 0) break; // Found a good list
                }
            }
            if (benefits.length > 5) benefits = benefits.slice(0, 8);

            // 3. Call to Action (Buttons)
            const ctas = Array.from(document.querySelectorAll('button, a.button, a.btn, [role="button"], input[type="submit"]'))
                .filter(el => {
                    const style = window.getComputedStyle(el);
                    return style.display !== 'none' && style.visibility !== 'hidden' && el.offsetWidth > 0;
                })
                .map(el => el.textContent.trim())
                .filter(t => t.length > 2 && t.length < 30)
                .slice(0, 5);

            return { headlines, key_benefits: benefits, ctas };
        })()
    "#;

    let result = tab.evaluate(script, false)?;
    
    // Safely deserializing result
    if let Some(value) = result.value {
        let data: MarketingData = serde_json::from_value(value)?;
             // Log findings
        println!("📢 Marketing Data: {} headlines, {} benefits, {} CTAs", 
            data.headlines.len(), data.key_benefits.len(), data.ctas.len());
            
        Ok(data)
    } else {
        println!("⚠️ Marketing extraction script returned no value.");
        Err(anyhow::anyhow!("No data returned from script"))
    }
}

// Public function to decode Bing/Google redirect URLs to get actual destination
pub fn decode_search_url(url: &str) -> String {
    // Bing URLs: https://www.bing.com/ck/a?...&u=a1aHR0c...
    if url.contains("bing.com/ck/a") {
        if let Some(u_param) = url.split("&u=").nth(1) {
            let encoded = u_param.split('&').next().unwrap_or(u_param);
            // Remove 'a1' prefix if present
            let base64_part = if encoded.starts_with("a1") {
                &encoded[2..]
            } else {
                encoded
            };
            // Decode base64
            if let Ok(decoded) = base64_decode(base64_part) {
                if let Ok(decoded_str) = String::from_utf8(decoded) {
                    println!("Decoded Bing URL: {}", decoded_str);
                    return decoded_str;
                }
            }
        }
    }
    // Google URLs: https://www.google.com/url?...&url=https...
    if url.contains("google.com/url") {
        if let Some(url_param) = url.split("&url=").nth(1).or_else(|| url.split("?url=").nth(1)) {
            let decoded_url = urlencoding::decode(url_param.split('&').next().unwrap_or(url_param))
                .unwrap_or_else(|_| url_param.into())
                .to_string();
            return decoded_url;
        }
    }
    // Return original if not a redirect URL
    url.to_string()
}

// Simple base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use std::collections::HashMap;
    
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut decode_map: HashMap<char, u8> = HashMap::new();
    for (i, c) in alphabet.chars().enumerate() {
        decode_map.insert(c, i as u8);
    }
    
    let input = input.trim_end_matches('=');
    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits_collected = 0;
    
    for c in input.chars() {
        if let Some(&val) = decode_map.get(&c) {
            buffer = (buffer << 6) | val as u32;
            bits_collected += 6;
            if bits_collected >= 8 {
                bits_collected -= 8;
                output.push((buffer >> bits_collected) as u8);
                buffer &= (1 << bits_collected) - 1;
            }
        }
    }
    
    Ok(output)
}

// ============================================================================
// Generic Forum Crawler
// ============================================================================
pub async fn generic_crawl(url: &str, selectors: Option<std::collections::HashMap<String, String>>) -> Result<SerpData> {
    println!("🌐 Starting Generic Crawl for: {}", url);
    use rand::seq::SliceRandom;
    
    // Minimal browser setup for brevity (reusing user agent list from top of file)
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng())
        .unwrap_or(&"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36");

    let args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--headless"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
    ];

    let browser = Browser::new(LaunchOptions {
        headless: true, 
        args,
        window_size: Some((1920, 1080)),
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    
    // Inject cookies if domain match found in cookies.json
    // Simple domain extraction for key lookup (e.g. "facebook.com")
    let domain_key = if url.contains("facebook.com") { "facebook.com" } 
                     else if url.contains("google.com") { "google.com" }
                     else { "unknown" };
                     
    if let Some(cookies) = load_cookies(domain_key) {
        let _ = inject_cookies(&tab, &cookies);
    }

    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    
    // Safety: Check for initial ban/checkpoint immediately after load
    if let Err(e) = check_for_ban(&tab) {
        println!("{}", e);
        return Err(e);
    }
    
    // Safety: Sleep before interaction
    safe_sleep().await;
    
    // Special handling for Facebook
    if url.contains("facebook.com") {
        println!("📘 Facebook Domain Detected. Engaging Human Scroll Mode...");
        scroll_safe(&tab).await?;
    } else {
        // Generic Scroll
        // Simulate scroll for forums (often lazy load)
        let _ = tab.evaluate("window.scrollTo(0, document.body.scrollHeight);", false);
        // Safety: Sleep after scroll
        safe_sleep().await;
    }

    // Capture verification screenshot (Critical for User Assurance)

    // Capture verification screenshot (Critical for User Assurance)
    println!("📸 Capturing Generic Verification Screenshot...");
    if let Ok(screenshot) = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None, None, true
    ) {
        let _ = std::fs::write("debug/debug_generic_stealth.png", &screenshot);
        println!("✅ Screenshot saved to debug/debug_generic_stealth.png");
    }

    let html_content = tab.get_content()?;
    let document = Html::parse_document(&html_content);
    
    let mut results = Vec::new();
    let mut snippet_acc = String::new();

    if let Some(sel_map) = selectors {
        for (key, selector_str) in sel_map {
             if let Ok(selector) = Selector::parse(&selector_str) {
                 snippet_acc.push_str(&format!("--- {} ---\n", key));
                 for element in document.select(&selector) {
                     snippet_acc.push_str(&element.text().collect::<String>());
                     snippet_acc.push('\n');
                 }
             }
        }
    } else {
        // Default: Extract Title + H1
        snippet_acc.push_str("No selectors provided. Dumping title.\n");
        let title_sel = Selector::parse("title").unwrap();
        if let Some(t) = document.select(&title_sel).next() {
            snippet_acc.push_str(&t.text().collect::<String>());
        }
    }

    results.push(SearchResult {
        title: "Forum Data".to_string(),
        link: url.to_string(),
        snippet: snippet_acc,
    });

    Ok(SerpData {
        results,
        total_results: Some("1".to_string()),
        ..Default::default()
    })
}








