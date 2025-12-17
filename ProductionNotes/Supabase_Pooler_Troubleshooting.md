# Supabase Pooler Troubleshooting & Architecture Guide

## 1. Troubleshooting Case Study: "Tenant Not Found"
**Date:** 2025-12-16
**Issue:** The Rust Crawler could not connect to Supabase, failing with `Tenant or user not found` despite good network connectivity.

### The Problem
The application was configured with the **wrong Pooler Host Alias**.
- **Incorrect:** `aws-0-ap-south-1.pooler.supabase.com`
- **Correct:** `aws-1-ap-south-1.pooler.supabase.com`

Supabase uses multiple "shards" (aws-0, aws-1, etc.) for their transaction poolers. Using the wrong shard means the specific "Tenant" (your project) cannot be found on that server even if the password is correct.

### The Fix
1.  Used the **Supabase Management API** (with a PAT token) to fetch the authoritative config:
    ```bash
    curl -H "Authorization: Bearer sbp_..." https://api.supabase.com/v1/projects/dvwpueeqhgqmivetcfwo/config/database/pooler
    ```
2.  Updated `.env` with the specific connection string returned by the API.

---

## 2. PostgreSQL vs. Supabase: What's the Difference?

You asked: *"What is the difference between PostgreSQL and Supabase, and why do we use it?"*

### 🐘 PostgreSQL (The Engine)
**PostgreSQL** is the actual open-source database software. It is a powerful, reliable "Relational Database Management System" (RDBMS). It stores your data in tables (rows and columns) and allows you to query it using SQL.

If you ran "just PostgreSQL", you would:
1.  Rent a Linux server (VPS).
2.  Install `sudo apt install postgresql`.
3.  Configure firewalls, backups, updates, and security manually.

### ⚡ Supabase (The Platform)
**Supabase** is a *platform* that **hosts** PostgreSQL for you, but then wraps it in a suite of incredible tools. It is often called an "Open Source Firebase Alternative."

When you use Supabase, you **ARE** using PostgreSQL, but you also get:
1.  **Managed Hosting**: They handle backups, upgrades, and scaling.
2.  **Connection Pooling (Supavisor)**: This was the key to our troubleshooting. It allows thousands of temporary connections (like from a crawler or serverless function) to share a small number of real database connections, preventing the database from crashing under load.
3.  **Data API**: It automatically builds a REST API for your database, so you can fetch data from frontend apps without writing backend code.
4.  **Auth & Realtime**: Built-in user handling and websocket subscriptions for live data updates.

### 🚀 Why We Use Supabase for the Crawler
1.  **Zero Maintenance**: We don't want to spend time patching Linux servers.
2.  **Transaction Pooling**: Our crawler runs on multiple threads (and potentially multiple machines via the distributed router setup). Opening 100+ direct connections to Postgres is heavy. Supabase's Transaction Pooler handles this traffic efficiently at port `6543`.
3.  **Scale**: If we scrape 1,000,000 pages, Supabase's infrastructure can handle the storage growth better than a cheap laptop hard drive.
