from bs4 import BeautifulSoup
import json

with open('debug_google_tier1.html', 'r', encoding='utf-8', errors='ignore') as f:
    html = f.read()

soup = BeautifulSoup(html, 'html.parser')

print(f"Total HTML size: {len(html)} bytes")

# Check for main content
main = soup.find(attrs={'role': 'main'}) or soup.find(id='main')
print(f"\n[role='main'] found: {main is not None}")

if main:
    # Find all elements with data-ved (these are Google result markers)
    ved_elements = main.find_all(attrs={'data-ved': True})
    print(f"Elements with data-ved in main: {len(ved_elements)}")
    
    # Check for various title containers
    print("\nSearching for title containers:")
    for selector in ['h3', 'h2', 'div[role="heading"]', 'span[role="heading"]', '.LC20lb', '.DKV0Md']:
        els = main.find_all(selector)
        print(f"  {selector}: {len(els)}")
        if els and len(els) > 0:
            print(f"    Example: {els[0].get_text()[:50] if els[0] else 'N/A'}")
    
    # Find all links
    links = main.find_all('a', href=True)
    http_links = [a for a in links if a['href'].startswith('http') and 'google.com' not in a['href']]
    print(f"\nTotal links in main: {len(links)}")
    print(f"External HTTP links: {len(http_links)}")
    if http_links:
        print(f"  First link: {http_links[0]['href'][:60]}")
        print(f"  Link text: {http_links[0].get_text()[:50]}")

# Check for script data
scripts = soup.find_all('script')
print(f"\n<script> tags: {len(scripts)}")
for i, script in enumerate(scripts[:5]):
    content = script.string or ''
    if 'AF_initDataCallback' in content or '"results"' in content:
        print(f"  Script {i} contains data structures (length: {len(content)})")
