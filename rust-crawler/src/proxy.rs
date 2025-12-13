//! Production-grade proxy rotation module
//! 
//! Supports:
//! - Authenticated proxies (user:pass@host:port)
//! - Multiple rotation strategies
//! - Health tracking with automatic failure recovery
//! - Runtime management

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

/// Global proxy manager instance
pub static PROXY_MANAGER: Lazy<ProxyManager> = Lazy::new(|| {
    let proxies_str = std::env::var("PROXY_LIST").unwrap_or_default();
    let strategy_str = std::env::var("PROXY_ROTATION").unwrap_or_else(|_| "roundrobin".to_string());
    let max_fails: u32 = std::env::var("PROXY_MAX_FAILS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let strategy = match strategy_str.to_lowercase().as_str() {
        "leastused" => RotationStrategy::LeastUsed,
        "random" => RotationStrategy::Random,
        "weighted" => RotationStrategy::Weighted,
        _ => RotationStrategy::RoundRobin,
    };

    let proxies: Vec<Arc<Proxy>> = proxies_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|s| Proxy::parse(s).ok())
        .map(Arc::new)
        .collect();

    if proxies.is_empty() {
        println!("📡 No proxies configured. Using direct connection.");
    } else {
        println!("📡 Loaded {} proxies with {:?} rotation strategy.", proxies.len(), strategy);
    }

    ProxyManager::new(proxies, strategy, max_fails)
});

/// Proxy protocol types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    Http,
    Https,
    Socks5,
}

impl Default for ProxyProtocol {
    fn default() -> Self {
        ProxyProtocol::Http
    }
}

/// Rotation strategy for proxy selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationStrategy {
    /// Simple round-robin rotation
    RoundRobin,
    /// Pick proxy with lowest request count
    LeastUsed,
    /// Random selection from healthy proxies
    Random,
    /// Higher success rate = higher priority
    Weighted,
}

/// Individual proxy configuration with stats
pub struct Proxy {
    /// Unique identifier
    pub id: String,
    /// Host address (without port)
    pub host: String,
    /// Port number
    pub port: u16,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Protocol type
    pub protocol: ProxyProtocol,
    /// Is proxy currently healthy?
    pub healthy: AtomicBool,
    /// Consecutive failure count
    pub fail_count: AtomicU32,
    /// Last used timestamp (unix seconds)
    pub last_used: AtomicI64,
    /// Total successful requests
    pub success_count: AtomicU64,
    /// Total requests made
    pub total_requests: AtomicU64,
}

impl Proxy {
    /// Parse proxy string in various formats:
    /// - `host:port`
    /// - `user:pass@host:port`
    /// - `protocol://user:pass@host:port`
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut s = s.trim();
        
        // Extract protocol if present
        let protocol = if s.starts_with("socks5://") {
            s = &s[9..];
            ProxyProtocol::Socks5
        } else if s.starts_with("https://") {
            s = &s[8..];
            ProxyProtocol::Https
        } else if s.starts_with("http://") {
            s = &s[7..];
            ProxyProtocol::Http
        } else {
            ProxyProtocol::Http
        };

        // Check for auth (user:pass@)
        let (auth, host_port) = if let Some(at_pos) = s.rfind('@') {
            let auth_part = &s[..at_pos];
            let host_part = &s[at_pos + 1..];
            (Some(auth_part), host_part)
        } else {
            (None, s)
        };

        // Parse username:password
        let (username, password) = if let Some(auth_str) = auth {
            if let Some(colon_pos) = auth_str.find(':') {
                (
                    Some(auth_str[..colon_pos].to_string()),
                    Some(auth_str[colon_pos + 1..].to_string()),
                )
            } else {
                return Err(format!("Invalid auth format (missing password): {}", s));
            }
        } else {
            (None, None)
        };

        // Parse host:port
        let (host, port) = if let Some(colon_pos) = host_port.rfind(':') {
            let host = host_port[..colon_pos].to_string();
            let port: u16 = host_port[colon_pos + 1..]
                .parse()
                .map_err(|_| format!("Invalid port: {}", &host_port[colon_pos + 1..]))?;
            (host, port)
        } else {
            return Err(format!("Missing port in proxy address: {}", host_port));
        };

        // Generate unique ID
        let id = format!("{}:{}", host, port);

        Ok(Self {
            id,
            host,
            port,
            username,
            password,
            protocol,
            healthy: AtomicBool::new(true),
            fail_count: AtomicU32::new(0),
            last_used: AtomicI64::new(0),
            success_count: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
        })
    }

    /// Get the Chrome proxy argument (--proxy-server=...)
    pub fn to_chrome_arg(&self) -> String {
        let protocol = match self.protocol {
            ProxyProtocol::Socks5 => "socks5",
            ProxyProtocol::Https => "https",
            ProxyProtocol::Http => "http",
        };
        format!("{}://{}:{}", protocol, self.host, self.port)
    }

    /// Check if proxy requires authentication
    pub fn requires_auth(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }

    /// Get success rate (0.0 - 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0; // Assume healthy if never used
        }
        let success = self.success_count.load(Ordering::Relaxed);
        success as f64 / total as f64
    }
}

/// Serializable proxy info for API responses
#[derive(Serialize, ToSchema)]
pub struct ProxyInfo {
    #[schema(example = "1.2.3.4:8080")]
    pub id: String,
    #[schema(example = "1.2.3.4")]
    pub host: String,
    #[schema(example = 8080)]
    pub port: u16,
    pub protocol: ProxyProtocol,
    pub has_auth: bool,
    pub healthy: bool,
    pub fail_count: u32,
    pub success_count: u64,
    pub total_requests: u64,
    pub success_rate: f64,
}

impl From<&Proxy> for ProxyInfo {
    fn from(p: &Proxy) -> Self {
        ProxyInfo {
            id: p.id.clone(),
            host: p.host.clone(),
            port: p.port,
            protocol: p.protocol,
            has_auth: p.requires_auth(),
            healthy: p.healthy.load(Ordering::Relaxed),
            fail_count: p.fail_count.load(Ordering::Relaxed),
            success_count: p.success_count.load(Ordering::Relaxed),
            total_requests: p.total_requests.load(Ordering::Relaxed),
            success_rate: p.success_rate(),
        }
    }
}

/// Aggregate stats for the proxy pool
#[derive(Serialize, ToSchema)]
pub struct ProxyStats {
    pub total_proxies: usize,
    pub healthy_proxies: usize,
    pub total_requests: u64,
    pub total_successes: u64,
    pub overall_success_rate: f64,
}

/// Proxy manager with rotation and health tracking
pub struct ProxyManager {
    proxies: RwLock<Vec<Arc<Proxy>>>,
    current_index: AtomicU64,
    strategy: RotationStrategy,
    max_fail_count: u32,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new(proxies: Vec<Arc<Proxy>>, strategy: RotationStrategy, max_fail_count: u32) -> Self {
        Self {
            proxies: RwLock::new(proxies),
            current_index: AtomicU64::new(0),
            strategy,
            max_fail_count,
        }
    }

    /// Get the next proxy based on rotation strategy
    pub fn get_next_proxy(&self) -> Option<Arc<Proxy>> {
        let proxies = self.proxies.read().ok()?;
        if proxies.is_empty() {
            return None;
        }

        // Filter to only healthy proxies
        let healthy: Vec<_> = proxies
            .iter()
            .filter(|p| p.healthy.load(Ordering::Relaxed))
            .collect();

        if healthy.is_empty() {
            println!("⚠️ All proxies unhealthy! Trying first proxy anyway...");
            return proxies.first().cloned();
        }

        let proxy = match self.strategy {
            RotationStrategy::RoundRobin => {
                let idx = self.current_index.fetch_add(1, Ordering::SeqCst) as usize % healthy.len();
                healthy[idx].clone()
            }
            RotationStrategy::LeastUsed => {
                healthy
                    .iter()
                    .min_by_key(|p| p.total_requests.load(Ordering::Relaxed))
                    .cloned()?
                    .clone()
            }
            RotationStrategy::Random => {
                use rand::seq::SliceRandom;
                healthy.choose(&mut rand::thread_rng())?.clone().clone()
            }
            RotationStrategy::Weighted => {
                // Simple weighted selection: pick highest success rate
                healthy
                    .iter()
                    .max_by(|a, b| {
                        a.success_rate()
                            .partial_cmp(&b.success_rate())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned()?
                    .clone()
            }
        };

        // Update last used timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        proxy.last_used.store(now, Ordering::Relaxed);
        proxy.total_requests.fetch_add(1, Ordering::Relaxed);

        Some(proxy)
    }

    /// Mark a proxy request as successful
    pub fn mark_success(&self, proxy_id: &str) {
        if let Ok(proxies) = self.proxies.read() {
            if let Some(proxy) = proxies.iter().find(|p| p.id == proxy_id) {
                proxy.success_count.fetch_add(1, Ordering::Relaxed);
                proxy.fail_count.store(0, Ordering::Relaxed);
                proxy.healthy.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Mark a proxy request as failed
    pub fn mark_failure(&self, proxy_id: &str) {
        if let Ok(proxies) = self.proxies.read() {
            if let Some(proxy) = proxies.iter().find(|p| p.id == proxy_id) {
                let fails = proxy.fail_count.fetch_add(1, Ordering::Relaxed) + 1;
                if fails >= self.max_fail_count {
                    println!("🚫 Proxy {} disabled after {} consecutive failures", proxy_id, fails);
                    proxy.healthy.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    /// Add a new proxy at runtime
    pub fn add_proxy(&self, proxy_str: &str) -> Result<ProxyInfo, String> {
        let proxy = Arc::new(Proxy::parse(proxy_str)?);
        let info = ProxyInfo::from(proxy.as_ref());
        
        if let Ok(mut proxies) = self.proxies.write() {
            // Check for duplicate
            if proxies.iter().any(|p| p.id == proxy.id) {
                return Err(format!("Proxy {} already exists", proxy.id));
            }
            println!("➕ Added proxy: {}", proxy.id);
            proxies.push(proxy);
        }
        
        Ok(info)
    }

    /// Remove a proxy by ID
    pub fn remove_proxy(&self, proxy_id: &str) -> Result<(), String> {
        if let Ok(mut proxies) = self.proxies.write() {
            let before_len = proxies.len();
            proxies.retain(|p| p.id != proxy_id);
            if proxies.len() == before_len {
                return Err(format!("Proxy {} not found", proxy_id));
            }
            println!("➖ Removed proxy: {}", proxy_id);
        }
        Ok(())
    }

    /// Re-enable a disabled proxy
    pub fn enable_proxy(&self, proxy_id: &str) -> Result<(), String> {
        if let Ok(proxies) = self.proxies.read() {
            if let Some(proxy) = proxies.iter().find(|p| p.id == proxy_id) {
                proxy.healthy.store(true, Ordering::Relaxed);
                proxy.fail_count.store(0, Ordering::Relaxed);
                println!("✅ Re-enabled proxy: {}", proxy_id);
                return Ok(());
            }
        }
        Err(format!("Proxy {} not found", proxy_id))
    }

    /// List all proxies with their stats
    pub fn list_proxies(&self) -> Vec<ProxyInfo> {
        if let Ok(proxies) = self.proxies.read() {
            proxies.iter().map(|p| ProxyInfo::from(p.as_ref())).collect()
        } else {
            Vec::new()
        }
    }

    /// Get aggregate stats
    pub fn get_stats(&self) -> ProxyStats {
        let proxies = self.proxies.read().ok();
        let (total, healthy, requests, successes) = proxies
            .map(|ps| {
                let total = ps.len();
                let healthy = ps.iter().filter(|p| p.healthy.load(Ordering::Relaxed)).count();
                let requests: u64 = ps.iter().map(|p| p.total_requests.load(Ordering::Relaxed)).sum();
                let successes: u64 = ps.iter().map(|p| p.success_count.load(Ordering::Relaxed)).sum();
                (total, healthy, requests, successes)
            })
            .unwrap_or((0, 0, 0, 0));

        ProxyStats {
            total_proxies: total,
            healthy_proxies: healthy,
            total_requests: requests,
            total_successes: successes,
            overall_success_rate: if requests > 0 {
                successes as f64 / requests as f64
            } else {
                1.0
            },
        }
    }

    /// Check if any proxies are configured
    pub fn has_proxies(&self) -> bool {
        self.proxies.read().map(|p| !p.is_empty()).unwrap_or(false)
    }
}

/// Generate Chrome extension for proxy authentication
/// This creates a minimal Chrome extension that intercepts proxy auth requests
pub fn generate_proxy_auth_extension(username: &str, password: &str) -> String {
    let manifest = r#"{
  "version": "1.0.0",
  "manifest_version": 2,
  "name": "Proxy Auth",
  "permissions": ["proxy", "webRequest", "webRequestBlocking", "<all_urls>"],
  "background": { "scripts": ["background.js"] }
}"#;

    let background = format!(
        r#"chrome.webRequest.onAuthRequired.addListener(
  function(details) {{
    return {{
      authCredentials: {{
        username: "{}",
        password: "{}"
      }}
    }};
  }},
  {{ urls: ["<all_urls>"] }},
  ["blocking"]
);"#,
        username.replace('\\', "\\\\").replace('"', "\\\""),
        password.replace('\\', "\\\\").replace('"', "\\\"")
    );

    // Return as base64 encoded CRX or directory path
    // For simplicity, we'll write to a temp directory
    let temp_dir = std::env::temp_dir().join("proxy_auth_ext");
    let _ = std::fs::create_dir_all(&temp_dir);
    let _ = std::fs::write(temp_dir.join("manifest.json"), manifest);
    let _ = std::fs::write(temp_dir.join("background.js"), background);
    
    temp_dir.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_proxy() {
        let proxy = Proxy::parse("192.168.1.1:8080").unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 8080);
        assert!(proxy.username.is_none());
        assert!(proxy.password.is_none());
    }

    #[test]
    fn test_parse_auth_proxy() {
        let proxy = Proxy::parse("user:pass@proxy.example.com:3128").unwrap();
        assert_eq!(proxy.host, "proxy.example.com");
        assert_eq!(proxy.port, 3128);
        assert_eq!(proxy.username, Some("user".to_string()));
        assert_eq!(proxy.password, Some("pass".to_string()));
    }

    #[test]
    fn test_parse_socks5_proxy() {
        let proxy = Proxy::parse("socks5://user:pass@127.0.0.1:1080").unwrap();
        assert_eq!(proxy.protocol, ProxyProtocol::Socks5);
        assert_eq!(proxy.host, "127.0.0.1");
        assert_eq!(proxy.port, 1080);
    }

    #[test]
    fn test_chrome_arg() {
        let proxy = Proxy::parse("http://proxy.example.com:8080").unwrap();
        assert_eq!(proxy.to_chrome_arg(), "http://proxy.example.com:8080");
    }
}
