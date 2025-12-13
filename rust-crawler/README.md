# Rust Crawler

Production-grade web crawler with **Google** and **Bing** search support, featuring proxy rotation, stealth mode, and search result extraction.

## Features

### Core Capabilities
- ✅ **Google Search** - First page results with exact match/verbatim support
- ✅ **Bing Search** - First page results with challenge detection
- ✅ **Content Extraction** - Fetches HTML from the first result
- ✅ **Stealth Mode** - Bypasses webdriver detection, canvas fingerprinting, WebGL

### Proxy Rotation (Production-Grade)
- ✅ **Authenticated proxies** - Support for `user:pass@host:port` format
- ✅ **4 Rotation Strategies** - RoundRobin, LeastUsed, Random, Weighted
- ✅ **Health tracking** - Auto-disables proxies after consecutive failures
- ✅ **Runtime management** - Add/remove/enable proxies via API

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/crawl` | Start a new crawl |
| `GET` | `/crawl/:task_id` | Get crawl results |
| `GET` | `/proxies` | List all proxies |
| `POST` | `/proxies` | Add proxy at runtime |
| `DELETE` | `/proxies/:id` | Remove proxy |
| `POST` | `/proxies/:id/enable` | Re-enable disabled proxy |
| `GET` | `/proxies/stats` | Aggregate proxy stats |

## Quick Start

### 1. Environment Setup
```bash
cp .env.example .env
# Edit .env with your DATABASE_URL
```

### 2. Run with Docker Compose
```bash
cd /home/guest/tzdump/crawling
docker-compose up -d
```

### 3. Run Locally (Development)
```bash
cd rust-crawler
source .env
cargo run
```

### 4. Test the API
```bash
# Bing Search
curl -X POST http://localhost:3000/crawl \
  -H "Content-Type: application/json" \
  -d '{"keyword": "your search term", "engine": "bing"}'

# Google Search
curl -X POST http://localhost:3000/crawl \
  -H "Content-Type: application/json" \
  -d '{"keyword": "your search term", "engine": "google"}'

# Check Results
curl http://localhost:3000/crawl/{task_id}
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `PROXY_LIST` | Comma-separated proxies | (empty = direct) |
| `PROXY_ROTATION` | roundrobin, leastused, random, weighted | roundrobin |
| `PROXY_MAX_FAILS` | Failures before proxy disabled | 3 |

### Proxy Format Examples
```bash
# Simple
PROXY_LIST="proxy1.com:8080,proxy2.com:3128"

# With authentication
PROXY_LIST="user:pass@premium-proxy.com:8080,user2:pass2@backup.com:3128"

# With protocol
PROXY_LIST="socks5://proxy.com:1080,http://user:pass@proxy2.com:8080"
```

## Output Files

Each crawl generates:
- `crawl-results/{keyword}_{engine}_{id}.json` - Search results and metadata
- `crawl-results/{keyword}_{engine}_{id}.html` - HTML of first result

### JSON Structure
```json
{
  "keyword": "search term",
  "engine": "bing",
  "websites": ["url1", "url2", ...],
  "results_count": 10,
  "first_page_html_file": "path/to/file.html"
}
```

## Directory Structure
```
rust-crawler/
├── src/
│   ├── main.rs       # API server and routes
│   ├── api.rs        # API handlers
│   ├── crawler.rs    # Search engine crawling logic
│   ├── db.rs         # Database operations
│   └── proxy.rs      # Proxy rotation module
├── debug/            # Debug screenshots and HTML
├── logs/             # Application logs
├── scripts/          # Helper scripts
├── crawl-results/    # Output files
├── Cargo.toml
├── Dockerfile
└── .env
```

## Key Implementation Details

### Search Box Clearing
Before each search, the search box is cleared using JavaScript to prevent stale queries:
```javascript
input.value = ''; input.focus();
```

### Non-Blocking Scroll (Bing)
Light scroll simulation that doesn't block the browser:
```javascript
setInterval(() => { window.scrollBy(0, 100); ... }, 150);
```

### Google Verbatim Search
Automatically clicks "Search instead for [exact term]" when Google autocorrects:
```javascript
document.querySelector('a.spell_orig')?.click();
```

## Changelog

### 2025-12-13
- Added production-grade proxy rotation with 4 strategies
- Fixed search box clearing between searches
- Fixed Bing timeout issues with non-blocking scroll
- Added proxy management API endpoints
- Organized directory structure (debug/, logs/, scripts/)
- Updated docker-compose.yml for Rust crawler only

## License
MIT
