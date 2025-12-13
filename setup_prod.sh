#!/bin/bash
SERVER_IP="54.179.175.198"
KEY_FILE="sg-crawling-key.pem"

echo "🚀 Setting up Production Server ($SERVER_IP)..."

# Fix permissions
chmod 400 $KEY_FILE

# install docker
ssh -o StrictHostKeyChecking=no -i $KEY_FILE ubuntu@$SERVER_IP << 'EOF'
    echo "📦 Installing Dependencies..."
    sudo apt-get update
    sudo apt-get install -y docker.io docker-compose git
    sudo usermod -aG docker ubuntu
    
    echo "📂 Cloning Repository..."
    # Clone public repo (or enter credentials manually if private)
    # Using https for public access
    git clone https://github.com/pretamane/crawling.git
    cd crawling
    
    echo "⚡ Building & Starting Containers on Powerful Instance..."
    # 8GB RAM is plenty for Rust compilation
    sudo docker-compose up -d --build rust-crawler
EOF

echo "✅ Deployment Triggered! Check http://$SERVER_IP:3000 in a few minutes."
