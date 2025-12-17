#!/bin/bash

# Configuration
API_URL="http://localhost:3000"
KEYWORD="https://example.com"
USER_ID="test-user-123"

# 1. Generate Token
echo "🔑 Generating Token..."
TOKEN=$(python3 ../generate_token.py)
echo "Token: $TOKEN"

# 2. Trigger Crawl
echo -e "\n🚀 Triggering Crawl..."
RESPONSE=$(curl -s -X POST "$API_URL/crawl" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"keyword\": \"$KEYWORD\", \"engine\": \"generic\"}")

echo "Response: $RESPONSE"
TASK_ID=$(echo $RESPONSE | jq -r '.task_id')

if [ "$TASK_ID" == "null" ]; then
  echo "❌ Failed to trigger crawl"
  exit 1
fi

echo "✅ Task ID: $TASK_ID"

# 3. Poll for Completion
echo -e "\n⏳ Polling for completion..."
MAX_RETRIES=30
for i in $(seq 1 $MAX_RETRIES); do
  STATUS_RES=$(curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/crawl/$TASK_ID")
  STATUS=$(echo $STATUS_RES | jq -r '.status')
  
  if [ "$STATUS" == "completed" ]; then
    echo "✅ Crawl Completed!"
    echo $STATUS_RES | jq .
    break
  fi
  
  echo "Current Status: $STATUS (Attempt $i/$MAX_RETRIES)"
  sleep 2
done

if [ "$STATUS" != "completed" ]; then
  echo "❌ Search timed out"
  exit 1
fi

# 4. Check Notifications
echo -e "\n🔔 Checking Notifications..."
NOTIF_RES=$(curl -s -H "Authorization: Bearer $TOKEN" "$API_URL/notifications")
echo $NOTIF_RES | jq .

# Check if we have a notification about this crawl
HAS_NOTIF=$(echo $NOTIF_RES | jq -r ".[] | select(.message | contains(\"$KEYWORD\")) | .id")

if [ -n "$HAS_NOTIF" ]; then
  echo "✅ Found notification for this crawl!"
else
  echo "❌ No notification found for this crawl."
fi

echo -e "\n🎉 End-to-End Test Verification Complete!"
