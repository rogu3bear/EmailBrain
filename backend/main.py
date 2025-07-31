from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field
from typing import Optional
import httpx
import logging
import sqlite3
import os
import json
from datetime import datetime

app = FastAPI(title="EmailBrain API", version="1.0.0")

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3001"],  # Frontend origin
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Database configuration
DB_PATH = os.path.join(os.path.dirname(__file__), "db", "mail.db")

def get_db_connection():
    """Create a connection to the SQLite database"""
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn

class ChatRequest(BaseModel):
    prompt: str
    adapter_id: Optional[int] = Field(None, description="ID of the LoRA adapter to use")

class EmailData(BaseModel):
    subject: str
    sender: str
    body: str
    date: str
    recipients: Optional[str] = None
    thread_id: Optional[str] = None


def get_adapter_by_id(adapter_id):
    """Retrieve adapter information from the database by ID"""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute("SELECT * FROM adapters WHERE id = ?", (adapter_id,))
        adapter = cursor.fetchone()
        conn.close()
        return adapter
    except sqlite3.Error as e:
        logger.error(f"Database error when retrieving adapter: {str(e)}")
        return None

def log_chat_interaction(prompt, response, tokens_in, tokens_out, adapter_id=None):
    """Log chat interaction to the database"""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute(
            "INSERT INTO logs (prompt, response, tokens_in, tokens_out, adapter_id) VALUES (?, ?, ?, ?, ?)",
            (prompt, json.dumps(response), tokens_in, tokens_out, adapter_id)
        )
        conn.commit()
        conn.close()
    except sqlite3.Error as e:
        logger.error(f"Database error when logging chat: {str(e)}")

def save_email_to_db(email_data: EmailData):
    """Save email data to the database"""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute(
            "INSERT INTO emails (subject, sender, body, date, recipients, thread_id) VALUES (?, ?, ?, ?, ?, ?)",
            (email_data.subject, email_data.sender, email_data.body, email_data.date, email_data.recipients, email_data.thread_id)
        )
        conn.commit()
        email_id = cursor.lastrowid
        conn.close()
        return email_id
    except sqlite3.Error as e:
        logger.error(f"Database error when saving email: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Database error occurred while saving email"
        )

@app.get("/api/v1/adapters")
async def list_adapters():
    """Endpoint to list all available LoRA adapters"""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute("SELECT id, name, path, train_tokens, created_at FROM adapters")
        adapters = [dict(row) for row in cursor.fetchall()]
        conn.close()
        return {"adapters": adapters}
    except sqlite3.Error as e:
        logger.error(f"Database error when listing adapters: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Database error occurred while retrieving adapters"
        )

@app.get("/api/v1/emails")
async def list_emails():
    """Endpoint to list all emails"""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute("SELECT id, subject, sender, date, recipients, thread_id FROM emails ORDER BY date DESC")
        emails = [dict(row) for row in cursor.fetchall()]
        conn.close()
        return {"emails": emails}
    except sqlite3.Error as e:
        logger.error(f"Database error when listing emails: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Database error occurred while retrieving emails"
        )

@app.get("/api/v1/emails/{email_id}")
async def get_email(email_id: int):
    """Endpoint to get a specific email by ID"""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute("SELECT * FROM emails WHERE id = ?", (email_id,))
        email = cursor.fetchone()
        conn.close()
        
        if not email:
            raise HTTPException(
                status_code=404,
                detail=f"Email with ID {email_id} not found"
            )
        
        return {"email": dict(email)}
    except sqlite3.Error as e:
        logger.error(f"Database error when retrieving email: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Database error occurred while retrieving email"
        )

@app.post("/api/v1/emails")
async def receive_email(email_data: EmailData):
    """Endpoint to receive email data from Swift app"""
    try:
        email_id = save_email_to_db(email_data)
        logger.info(f"Received and saved email: {email_data.subject} from {email_data.sender}")
        return {
            "message": "Email received successfully",
            "email_id": email_id,
            "email": email_data.dict()
        }
    except Exception as e:
        logger.error(f"Error processing email data: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Internal server error occurred while processing email"
        )

@app.post("/api/v1/chat")
async def chat(chat_request: ChatRequest):
    adapter = None
    
    # Check if adapter_id is provided and valid
    if chat_request.adapter_id is not None:
        adapter = get_adapter_by_id(chat_request.adapter_id)
        if not adapter:
            raise HTTPException(
                status_code=404,
                detail=f"LoRA adapter with ID {chat_request.adapter_id} not found"
            )
    
    try:
        # Prepare request payload
        payload = {
            "model": "local-model",
            "messages": [{"role": "user", "content": chat_request.prompt}],
            "temperature": 0.7,
            "max_tokens": 150
        }
        
        # Add LoRA adapter configuration if an adapter is specified
        if adapter:
            adapter_path = os.path.join(os.path.dirname(os.path.dirname(__file__)), adapter['path'])
            if not os.path.exists(adapter_path):
                raise HTTPException(
                    status_code=404,
                    detail=f"LoRA adapter file not found at {adapter_path}"
                )
            
            # Add LoRA configuration to the payload
            payload["lora"] = {
                "adapter_path": adapter_path
            }
            logger.info(f"Using LoRA adapter: {adapter['name']} (ID: {adapter['id']})")
        
        async with httpx.AsyncClient(timeout=30.0) as client:
            # Use the correct LM Studio chat completions endpoint
            response = await client.post(
                "http://127.0.0.1:1234/v1/chat/completions",
                json=payload,
            )
            
            if response.status_code != 200:
                logger.error(f"LM Studio API error: {response.status_code} - {response.text}")
                raise HTTPException(
                    status_code=response.status_code,
                    detail=f"LM Studio API error: {response.text}"
                )
            
            response_data = response.json()
            
            # Log the interaction in the database
            tokens_in = len(chat_request.prompt.split())  # Simple approximation
            tokens_out = 0
            if "choices" in response_data and len(response_data["choices"]) > 0:
                if "message" in response_data["choices"][0] and "content" in response_data["choices"][0]["message"]:
                    tokens_out = len(response_data["choices"][0]["message"]["content"].split())
            
            log_chat_interaction(
                chat_request.prompt,
                response_data,
                tokens_in,
                tokens_out,
                chat_request.adapter_id
            )
            
            return response_data
            
    except httpx.ConnectError:
        logger.error("Failed to connect to LM Studio server at http://127.0.0.1:1234")
        raise HTTPException(
            status_code=503,
            detail="Unable to connect to LM Studio server. Please ensure it's running on http://127.0.0.1:1234"
        )
    except httpx.TimeoutException:
        logger.error("Request to LM Studio server timed out")
        raise HTTPException(
            status_code=504,
            detail="Request to LM Studio server timed out"
        )
    except sqlite3.Error as e:
        logger.error(f"Database error during chat request: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Database error occurred while processing the chat request"
        )
    except Exception as e:
        logger.error(f"Unexpected error during chat request: {str(e)}")
        raise HTTPException(
            status_code=500,
            detail="Internal server error occurred while processing the chat request"
        )


@app.get("/health")
async def health():
    return {"status": "ok"}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
