import spacy
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel
from typing import List, Dict, Optional
import logging
from sklearn.pipeline import Pipeline
from sklearn.feature_extraction.text import CountVectorizer
from sklearn.naive_bayes import MultinomialNB
import numpy as np

# Configure Logging
logger = logging.getLogger(__name__)

router = APIRouter(prefix="/ml", tags=["Machine Learning"])

# --- Global Models (Lazy Loaded) ---
nlp = None
classifier = None

def load_models():
    """
    Loads the spaCy model and trains a dummy classifier on startup.
    """
    global nlp, classifier
    
    # 1. Load spaCy
    try:
        if nlp is None:
            logger.info("Loading spaCy model 'en_core_web_sm'...")
            nlp = spacy.load("en_core_web_sm")
            logger.info("spaCy model loaded successfully.")
    except OSError:
        logger.error("spaCy model 'en_core_web_sm' not found. Please run: python -m spacy download en_core_web_sm")
    except Exception as e:
        logger.error(f"Error loading spaCy: {e}")

    # 2. Train Dummy Classifier (Text -> Category)
    # in a real app, you would load a pickled model here.
    if classifier is None:
        logger.info("Training dummy classifier...")
        X_train = [
            "Apple released a new iPhone today", "Google updates search algorithm", "Python release notes", # Tech
            "Stock markets hit record high", "Fed raises interest rates", "Bitcoin price drops",            # Finance
            "New vaccine approved by FDA", "Benefits of meditation", "COVID-19 case numbers",               # Health
            "Manchester United wins match", "NBA finals scores", "Olympics gold medal"                      # Sports
        ]
        y_train = ["Tech", "Tech", "Tech", "Finance", "Finance", "Finance", "Health", "Health", "Health", "Sports", "Sports", "Sports"]
        
        pipeline = Pipeline([
            ('vect', CountVectorizer()),
            ('clf', MultinomialNB()),
        ])
        pipeline.fit(X_train, y_train)
        classifier = pipeline
        logger.info("Dummy classifier trained.")

# --- Pydantic Models ---

class TextRequest(BaseModel):
    text: str

class Entity(BaseModel):
    text: str
    label: str

class NERResponse(BaseModel):
    entities: List[Entity]

class ClassificationResponse(BaseModel):
    category: str
    confidence: float

# --- Endpoints ---

@router.on_event("startup")
async def startup_event():
    load_models()

@router.post("/ner", response_model=NERResponse)
async def extract_entities(request: TextRequest):
    """
    Extracts Named Entities (Person, Org, Location) from text using spaCy.
    """
    if nlp is None:
        # Try loading again just in case
        load_models()
        if nlp is None:
             raise HTTPException(status_code=503, detail="ML Model not loaded")

    doc = nlp(request.text)
    
    entities = [
        Entity(text=ent.text, label=ent.label_) 
        for ent in doc.ents 
        # Filter for common useful entities to reduce noise
        if ent.label_ in ["PERSON", "ORG", "GPE", "LOC", "PRODUCT", "EVENT", "DATE", "MONEY"]
    ]
    
    return NERResponse(entities=entities)

@router.post("/classify", response_model=ClassificationResponse)
async def classify_text(request: TextRequest):
    """
    Classifies text into categories (Tech, Finance, Health, Sports) using a Naive Bayes classifier.
    """
    if classifier is None:
         load_models()
         if classifier is None:
             raise HTTPException(status_code=503, detail="Classifier not loaded")

    # Predict
    category = classifier.predict([request.text])[0]
    
    # Get confidence (probability)
    probs = classifier.predict_proba([request.text])[0]
    confidence = float(np.max(probs))
    
    return ClassificationResponse(category=category, confidence=confidence)
