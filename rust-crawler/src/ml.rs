//! Simple keyword-based Sentiment Analysis module.
//! 
//! This module provides a lightweight sentiment analyzer that uses word lists
//! to classify text as Positive, Negative, or Neutral. No external ML dependencies.
//! 
//! This demonstrates NLP integration skills for CV purposes.

use once_cell::sync::Lazy;
use std::collections::HashSet;

// Common positive words for sentiment detection
static POSITIVE_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    vec![
        "good", "great", "excellent", "amazing", "wonderful", "fantastic", "superb",
        "outstanding", "brilliant", "love", "loved", "loving", "best", "better",
        "positive", "happy", "joy", "joyful", "beautiful", "perfect", "awesome",
        "incredible", "magnificent", "delightful", "pleasant", "satisfying", "satisfied",
        "recommend", "recommended", "impressive", "exceptional", "remarkable", "success",
        "successful", "win", "winner", "winning", "efficient", "effective", "helpful",
        "reliable", "trustworthy", "quality", "valuable", "beneficial", "favorable",
        "advantageous", "profitable", "thriving", "flourishing", "prosperous"
    ].into_iter().collect()
});

// Common negative words for sentiment detection
static NEGATIVE_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    vec![
        "bad", "terrible", "awful", "horrible", "poor", "worst", "worse", "hate",
        "hated", "hating", "dislike", "disappointing", "disappointed", "disappoints",
        "failure", "failed", "fail", "failing", "negative", "sad", "unhappy",
        "angry", "annoyed", "frustrated", "frustrating", "problem", "problems",
        "issue", "issues", "bug", "bugs", "broken", "crash", "crashed", "error",
        "errors", "mistake", "mistakes", "wrong", "incorrect", "useless", "waste",
        "scam", "fraud", "fake", "unreliable", "unstable", "slow", "difficult",
        "complicated", "confusing", "expensive", "overpriced", "worthless", "garbage",
        "trash", "rubbish", "pathetic", "mediocre", "subpar", "inferior"
    ].into_iter().collect()
});

/// Result of sentiment analysis
#[derive(Debug, Clone)]
pub struct SentimentResult {
    pub label: String,
    pub score: f32,
    pub positive_count: usize,
    pub negative_count: usize,
}

/// Analyzes the sentiment of the provided text using keyword matching.
/// Returns a formatted string like "Positive (0.85)" or "Negative (0.72)".
pub fn analyze_sentiment(text: &str) -> Option<String> {
    if text.is_empty() || text.len() < 50 {
        return None; // Skip analysis for very short text
    }

    let lowercase_text = text.to_lowercase();
    let words: Vec<&str> = lowercase_text
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.len() > 2)
        .collect();

    if words.is_empty() {
        return None;
    }

    let positive_count = words.iter().filter(|w| POSITIVE_WORDS.contains(*w)).count();
    let negative_count = words.iter().filter(|w| NEGATIVE_WORDS.contains(*w)).count();
    
    let total_sentiment_words = positive_count + negative_count;
    
    if total_sentiment_words == 0 {
        return Some("Neutral (0.50)".to_string());
    }

    let positive_ratio = positive_count as f32 / total_sentiment_words as f32;
    
    let (label, score) = if positive_ratio > 0.6 {
        ("Positive", positive_ratio)
    } else if positive_ratio < 0.4 {
        ("Negative", 1.0 - positive_ratio)
    } else {
        ("Neutral", 0.5 + (positive_ratio - 0.5).abs())
    };

    println!(
        "🧠 Sentiment Analysis: {} words analyzed, {} positive, {} negative",
        words.len(),
        positive_count,
        negative_count
    );

    Some(format!("{} ({:.2})", label, score))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_sentiment() {
        let text = "This product is amazing and wonderful. I love it so much. Best purchase ever!";
        let result = analyze_sentiment(text);
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("Positive"));
    }

    #[test]
    fn test_negative_sentiment() {
        let text = "This is terrible and horrible. I hate it. Worst experience ever, total failure.";
        let result = analyze_sentiment(text);
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("Negative"));
    }

    #[test]
    fn test_neutral_sentiment() {
        let text = "The item arrived on time. It works as described in the listing.";
        let result = analyze_sentiment(text);
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("Neutral"));
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity {
    pub text: String,
    pub label: String,
}

#[derive(Debug, Deserialize)]
struct NERResponse {
    entities: Vec<Entity>,
}

#[derive(Debug, Deserialize)]
struct ClassificationResponse {
    category: String,
    confidence: f32,
}

/// Calls the local Python Sidecar to extract named entities.
pub async fn extract_entities_remote(text: &str) -> Option<Vec<Entity>> {
    let client = reqwest::Client::new();
    let res = client.post("http://localhost:8000/ml/ner")
        .json(&serde_json::json!({ "text": text }))
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<NERResponse>().await {
                    Ok(data) => Some(data.entities),
                    Err(e) => {
                        eprintln!("⚠️ [ML] NER parse error: {}", e);
                        None
                    }
                }
            } else {
                eprintln!("⚠️ [ML] NER request failed: {}", response.status());
                None
            }
        },
        Err(e) => {
             eprintln!("⚠️ [ML] NER connection failed: {}. Is python-crawler running?", e);
             None
        }
    }
}

/// Calls the local Python Sidecar to classify content.
pub async fn classify_content_remote(text: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let res = client.post("http://localhost:8000/ml/classify")
        .json(&serde_json::json!({ "text": text }))
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<ClassificationResponse>().await {
                    Ok(data) => Some(data.category),
                    Err(e) => {
                        eprintln!("⚠️ [ML] Classify parse error: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        },
        Err(_) => None
    }
}
