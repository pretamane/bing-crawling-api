from bs4 import BeautifulSoup
import sys

try:
    with open('debug_google_tier1.html', 'r', encoding='utf-8') as f:
        html = f.read()
except UnicodeDecodeError:
    with open('debug_google_tier1.html', 'r', encoding='latin-1') as f:
        html = f.read()

soup = BeautifulSoup(html, 'html.parser')

print("Searching for 'Pretamane'...")
# Search in text
for element in soup.find_all(string=lambda text: text and "Pretamane" in text):
    print(f"Found in TEXT: {element.strip()[:50]}...")
    print(f"  Tag: {element.parent.name}")

# Search in scripts
for script in soup.find_all('script'):
    if script.string and "Pretamane" in script.string:
        print("Found in SCRIPT tag!")
        print(f"  Snippet: {script.string[:100]}...")

# Search in attributes (e.g. links)
for tag in soup.find_all(True):
    for attr, value in tag.attrs.items():
        if isinstance(value, str) and "Pretamane" in value:
            print(f"Found in ATTRIBUTE {attr}: {value[:50]}...")
            print(f"  Tag: {tag.name}")
