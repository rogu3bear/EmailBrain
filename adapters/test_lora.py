#!/usr/bin/env python3
"""
LoRA Adapter Testing Script

This script tests if the AI can successfully use a LoRA adapter by:
1. Listing available LoRA adapters from the database
2. Allowing the user to select an adapter
3. Sending a test prompt to the AI with the selected adapter
4. Displaying the response
"""

import os
import sys
import sqlite3
import argparse
import logging
import httpx
import asyncio
from typing import List, Dict, Any, Optional

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
DB_PATH = os.path.join(PROJECT_ROOT, "backend", "db", "mail.db")

sys.path.append(PROJECT_ROOT)
try:
    from backend.config import settings
except Exception:  # pragma: no cover - optional dependency path
    settings = None

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    handlers=[logging.StreamHandler()],
)
logger = logging.getLogger(__name__)

API_BASE_URL = (
    str(settings.EMAILBRAIN_API_URL).rstrip("/")
    if settings is not None
    else os.environ.get("EMAILBRAIN_API_URL", "http://127.0.0.1:3901").rstrip("/")
)

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Test a LoRA adapter with the AI")
    parser.add_argument(
        "--adapter_id", 
        type=int, 
        help="ID of the adapter to test (if not provided, will list available adapters)"
    )
    parser.add_argument(
        "--prompt", 
        type=str, 
        default="Summarize the key points from the last email you received.",
        help="Prompt to send to the AI"
    )
    
    return parser.parse_args()

def get_db_connection():
    """Create a connection to the SQLite database."""
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn

def list_adapters() -> List[Dict[str, Any]]:
    """List all available LoRA adapters from the database."""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute("SELECT id, name, path, train_tokens, created_at FROM adapters ORDER BY created_at DESC")
        adapters = [dict(row) for row in cursor.fetchall()]
        conn.close()
        return adapters
    except sqlite3.Error as e:
        logger.error(f"Database error when listing adapters: {e}")
        return []

def get_adapter_by_id(adapter_id: int) -> Optional[Dict[str, Any]]:
    """Get a specific adapter by ID."""
    try:
        conn = get_db_connection()
        cursor = conn.cursor()
        cursor.execute("SELECT id, name, path, train_tokens, created_at FROM adapters WHERE id = ?", (adapter_id,))
        adapter = cursor.fetchone()
        conn.close()
        return dict(adapter) if adapter else None
    except sqlite3.Error as e:
        logger.error(f"Database error when retrieving adapter: {e}")
        return None

async def test_adapter_with_ai(adapter_id: int, prompt: str) -> Dict[str, Any]:
    """Test a LoRA adapter by sending a prompt to the AI."""
    try:
        # Get the adapter details
        adapter = get_adapter_by_id(adapter_id)
        if not adapter:
            raise ValueError(f"Adapter with ID {adapter_id} not found")
        
        logger.info(f"Testing adapter: {adapter['name']} (ID: {adapter_id})")
        
        # Prepare the request payload
        payload = {
            "prompt": prompt,
            "adapter_id": adapter_id
        }
        
        # Send the request to the backend API
        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.post(
                f"{API_BASE_URL}/api/v1/chat",
                json=payload,
            )
            
            if response.status_code != 200:
                logger.error(f"API error: {response.status_code} - {response.text}")
                raise ValueError(f"API error: {response.text}")
            
            return response.json()
    
    except httpx.ConnectError:
        logger.error(f"Failed to connect to API at {API_BASE_URL}")
        raise ConnectionError(f"Failed to connect to API at {API_BASE_URL}")
    except Exception as e:
        logger.error(f"Error testing adapter: {e}")
        raise

def display_adapters(adapters: List[Dict[str, Any]]):
    """Display a list of available adapters."""
    if not adapters:
        print("No LoRA adapters found in the database.")
        return
    
    print("\nAvailable LoRA Adapters:")
    print("-" * 80)
    print(f"{'ID':<5} {'Name':<30} {'Created At':<20} {'Train Tokens':<15} {'Path'}")
    print("-" * 80)
    
    for adapter in adapters:
        print(f"{adapter['id']:<5} {adapter['name'][:30]:<30} {adapter['created_at'][:19]:<20} {adapter['train_tokens']:<15} {adapter['path']}")
    
    print("-" * 80)

def display_response(response: Dict[str, Any]):
    """Display the AI response."""
    print("\nAI Response:")
    print("-" * 80)
    
    if "choices" in response and len(response["choices"]) > 0:
        for index, choice in enumerate(response["choices"], start=1):
            if "message" in choice and "content" in choice["message"]:
                if len(response["choices"]) > 1:
                    print(f"[choice {index}]")
                print(choice["message"]["content"])
            else:
                print("No content in response")
    else:
        print("No choices in response")
    
    print("-" * 80)

async def main():
    """Main function to test a LoRA adapter."""
    args = parse_args()
    
    # List available adapters
    adapters = list_adapters()
    
    # If no adapter ID is provided, display the list and prompt for selection
    adapter_id = args.adapter_id
    if adapter_id is None:
        display_adapters(adapters)
        
        if not adapters:
            logger.error("No adapters available to test")
            sys.exit(1)
        
        try:
            adapter_id = int(input("\nEnter the ID of the adapter to test: "))
        except ValueError:
            logger.error("Invalid adapter ID")
            sys.exit(1)
    
    # Get the adapter
    adapter = get_adapter_by_id(adapter_id)
    if not adapter:
        logger.error(f"Adapter with ID {adapter_id} not found")
        sys.exit(1)
    
    # Test the adapter
    try:
        print(f"\nTesting adapter '{adapter['name']}' with prompt: '{args.prompt}'")
        response = await test_adapter_with_ai(adapter_id, args.prompt)
        display_response(response)
    except Exception as e:
        logger.error(f"Error testing adapter: {e}")
        sys.exit(1)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nOperation cancelled by user")
        sys.exit(0)
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        sys.exit(1)
