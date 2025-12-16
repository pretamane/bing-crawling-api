use headless_chrome::{Browser, LaunchOptions, protocol::cdp::Emulation::{SetTimezoneOverride, SetLocaleOverride}};
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🕵️ Starting Stealth Debugger...");
    
    // Exact args from crawler.rs
    let mut args = vec![
        std::ffi::OsStr::new("--disable-blink-features=AutomationControlled"),
        std::ffi::OsStr::new("--no-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--disable-infobars"),
        std::ffi::OsStr::new("--window-position=0,0"),
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--ignore-certificate-errors-spki-list"),
        std::ffi::OsStr::new("--incognito"),
        std::ffi::OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36"),
        std::ffi::OsStr::new("--headless=new"),
    ];

    let browser = Browser::new(LaunchOptions {
        headless: false, // Usage via args
        window_size: Some((1920, 1080)),
        args,
        ..Default::default()
    })?;

    let tab = browser.new_tab()?;
    
    // Apply Overrides for verification
    println!("Applying Stealth Overrides...");
    tab.call_method(SetTimezoneOverride { timezone_id: "Asia/Yangon".to_string() })?;
    tab.call_method(SetLocaleOverride { locale: Some("en-US".to_string()) })?;

    // Inject Stealth Script (Manual copy or import if possible, for now hardcoding a simple version to match logic)
    // We can't easily import crate::stealth from bin unless it's in lib. 
    // Assuming rust-crawler exposes lib logic or we just verify basic props.
    // Ideally we'd use the real one, but let's just test the browser fingerprint first.
    
    // Navigate to Google to see what's happening
    println!("Navigating to Google...");
    tab.navigate_to("https://www.google.com/?hl=en")?;
    tab.wait_until_navigated()?;
    
    sleep(Duration::from_secs(5)).await;

    println!("Capturing Google Screenshot...");
    let screenshot = tab.capture_screenshot(
        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
        None, None, true
    )?;
    std::fs::write("debug_google.png", screenshot)?;
    println!("Saved debug_google.png");
    
    // Check IP
    println!("Checking IP via ipinfo.io...");
    tab.navigate_to("https://ipinfo.io/json")?;
    tab.wait_until_navigated()?;
    let content = tab.get_content()?;
    println!("IP Info: {}", content);

    // Check Timezone
    println!("Checking resolved Timezone...");
    let tz_result = tab.evaluate("Intl.DateTimeFormat().resolvedOptions().timeZone", false)?;
    println!("Timezone: {:?}", tz_result.value);

    // Keep alive briefly
    sleep(Duration::from_secs(2)).await;

    Ok(())
}
