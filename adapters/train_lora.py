#!/usr/bin/env python3
"""
LoRA Adapter Training Script for Email Data

This script trains a LoRA (Low-Rank Adaptation) adapter based on extracted email data.
It reads the JSON file created by the EmailExtractor, formats the data for training,
trains a LoRA adapter using the PEFT library, and registers the adapter in the database.
"""

import os
import sys
import json
import glob
import argparse
import logging
import sqlite3
from datetime import datetime
from typing import Dict, Any, Optional

import torch
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    DataCollatorForLanguageModeling,
    Trainer,
    TrainingArguments,
)
from peft import get_peft_model, LoraConfig, TaskType, prepare_model_for_kbit_training
from datasets import Dataset

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
DATA_DIR = os.path.join(SCRIPT_DIR, "data")
MODELS_DIR = os.path.join(SCRIPT_DIR, "models")
DB_PATH = os.path.join(PROJECT_ROOT, "backend", "db", "mail.db")

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    handlers=[logging.StreamHandler()],
)
logger = logging.getLogger(__name__)

# Constants
DEFAULT_MODEL_PATH = os.path.join(PROJECT_ROOT, "data", "models", "phi-3-mini.gguf")
DEFAULT_LORA_R = 8
DEFAULT_LORA_ALPHA = 16
DEFAULT_LORA_DROPOUT = 0.05
DEFAULT_NUM_EPOCHS = 3
DEFAULT_LEARNING_RATE = 3e-4
DEFAULT_BATCH_SIZE = 4
DEFAULT_MAX_LENGTH = 512

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Train a LoRA adapter from email data")
    parser.add_argument(
        "--data_file", 
        type=str, 
        help="Path to the JSON file containing email data"
    )
    parser.add_argument(
        "--model_path", 
        type=str, 
        default=DEFAULT_MODEL_PATH,
        help=f"Path to the base model (default: {DEFAULT_MODEL_PATH})"
    )
    parser.add_argument(
        "--output_dir", 
        type=str, 
        default=MODELS_DIR,
        help="Directory to save the trained adapter"
    )
    parser.add_argument(
        "--lora_r", 
        type=int, 
        default=DEFAULT_LORA_R,
        help=f"LoRA attention dimension (default: {DEFAULT_LORA_R})"
    )
    parser.add_argument(
        "--lora_alpha", 
        type=int, 
        default=DEFAULT_LORA_ALPHA,
        help=f"LoRA alpha parameter (default: {DEFAULT_LORA_ALPHA})"
    )
    parser.add_argument(
        "--lora_dropout", 
        type=float, 
        default=DEFAULT_LORA_DROPOUT,
        help=f"LoRA dropout probability (default: {DEFAULT_LORA_DROPOUT})"
    )
    parser.add_argument(
        "--num_epochs", 
        type=int, 
        default=DEFAULT_NUM_EPOCHS,
        help=f"Number of training epochs (default: {DEFAULT_NUM_EPOCHS})"
    )
    parser.add_argument(
        "--learning_rate", 
        type=float, 
        default=DEFAULT_LEARNING_RATE,
        help=f"Learning rate (default: {DEFAULT_LEARNING_RATE})"
    )
    parser.add_argument(
        "--batch_size", 
        type=int, 
        default=DEFAULT_BATCH_SIZE,
        help=f"Training batch size (default: {DEFAULT_BATCH_SIZE})"
    )
    parser.add_argument(
        "--max_length", 
        type=int, 
        default=DEFAULT_MAX_LENGTH,
        help=f"Maximum sequence length (default: {DEFAULT_MAX_LENGTH})"
    )
    parser.add_argument(
        "--adapter_name", 
        type=str, 
        help="Name for the trained adapter (default: generated from email subject)"
    )
    
    return parser.parse_args()

def find_latest_data_file() -> Optional[str]:
    """Find the most recent email data JSON file if none is specified."""
    data_files = glob.glob(os.path.join(DATA_DIR, "*.json"))
    if not data_files:
        return None
    
    # Sort by modification time (newest first)
    data_files.sort(key=lambda x: os.path.getmtime(x), reverse=True)
    return data_files[0]

def load_email_data(file_path: str) -> Dict[str, Any]:
    """Load the email data from the JSON file."""
    try:
        with open(file_path, "r") as f:
            data = json.load(f)
        return data
    except (json.JSONDecodeError, FileNotFoundError) as e:
        logger.error(f"Error loading email data: {e}")
        raise

def prepare_training_data(email_data: Dict[str, Any]) -> Dataset:
    """Prepare the email data for training."""
    # Extract training examples from the email data
    training_examples = []
    has_structured_examples = False

    if "training_examples" in email_data:
        for example in email_data["training_examples"]:
            instruction = example.get("instruction", "").strip()
            input_text = example.get("input", "").strip()
            output_text = example.get("output", "").strip()
            if not output_text:
                continue

            prompt = f"### Instruction: {instruction}"
            if input_text:
                prompt += f"\n\n### Input: {input_text}"
            prompt += f"\n\n### Response: {output_text}"
            training_examples.append({"text": prompt})
            has_structured_examples = True
    
    # Check if the data has the expected structure
    if not has_structured_examples and "trainingData" in email_data and "contextPairs" in email_data["trainingData"]:
        # Add context pairs
        for pair in email_data["trainingData"]["contextPairs"]:
            training_examples.append({
                "text": f"### Instruction: {pair['input']}\n\n### Response: {pair['output']}"
            })
    
    # Add the main email content as a training example
    subject = ""
    sender = ""
    body = ""
    if "emailData" in email_data:
        subject = email_data["emailData"].get("subject", "")
        sender = email_data["emailData"].get("sender", "")
        body = email_data["emailData"].get("body", "")
    elif "metadata" in email_data:
        subject = email_data["metadata"].get("subject", "")
        sender = email_data["metadata"].get("sender", "")
        body = email_data.get("content", "")

    if subject or sender or body:
        
        # Example 1: Summarize the email
        training_examples.append({
            "text": f"### Instruction: Summarize this email from {sender} with subject '{subject}'\n\n### Response: This is an email from {sender} about {subject}. The main points are: {body[:100]}..."
        })
        
        # Example 2: Answer questions about the email
        training_examples.append({
            "text": f"### Instruction: Who sent the email with subject '{subject}'?\n\n### Response: The email was sent by {sender}."
        })
        
        # Example 3: Generate a response to the email
        training_examples.append({
            "text": f"### Instruction: Write a response to this email:\nFrom: {sender}\nSubject: {subject}\n\n{body[:200]}\n\n### Response: Thank you for your email regarding {subject}. I have received your message and will respond shortly."
        })
    if not training_examples:
        raise ValueError("Email data did not contain any usable training examples")

    # Create a Hugging Face dataset
    return Dataset.from_list(training_examples)

def train_lora_adapter(
    dataset: Dataset,
    model_path: str,
    output_dir: str,
    lora_r: int,
    lora_alpha: int,
    lora_dropout: float,
    num_epochs: int,
    learning_rate: float,
    batch_size: int,
    max_length: int,
) -> int:
    """Train a LoRA adapter on the email data."""
    # Load the base model and tokenizer
    logger.info(f"Loading base model from {model_path}")
    model = AutoModelForCausalLM.from_pretrained(
        model_path,
        torch_dtype=torch.float16,
        device_map="auto",
    )
    tokenizer = AutoTokenizer.from_pretrained(model_path)
    
    # Prepare the model for k-bit training
    model = prepare_model_for_kbit_training(model)
    
    # Define the LoRA configuration
    peft_config = LoraConfig(
        task_type=TaskType.CAUSAL_LM,
        inference_mode=False,
        r=lora_r,
        lora_alpha=lora_alpha,
        lora_dropout=lora_dropout,
        target_modules=["q_proj", "v_proj", "k_proj", "o_proj"],
    )
    
    # Get the PEFT model
    model = get_peft_model(model, peft_config)
    
    # Tokenize the dataset
    def tokenize_function(examples):
        return tokenizer(
            examples["text"],
            padding="max_length",
            truncation=True,
            max_length=max_length,
            return_tensors="pt",
        )
    
    tokenized_dataset = dataset.map(tokenize_function, batched=True)
    
    # Define training arguments
    training_args = TrainingArguments(
        output_dir=output_dir,
        num_train_epochs=num_epochs,
        per_device_train_batch_size=batch_size,
        gradient_accumulation_steps=4,
        learning_rate=learning_rate,
        weight_decay=0.01,
        warmup_steps=100,
        logging_steps=10,
        save_strategy="epoch",
        evaluation_strategy="no",
        fp16=True,
    )
    
    # Train the model
    logger.info("Starting LoRA adapter training")
    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=tokenized_dataset,
        data_collator=DataCollatorForLanguageModeling(tokenizer=tokenizer, mlm=False),
    )
    
    trainer.train()
    
    # Save the trained adapter
    logger.info(f"Saving LoRA adapter to {output_dir}")
    model.save_pretrained(output_dir)
    tokenizer.save_pretrained(output_dir)
    
    # Calculate approximate number of tokens used in training
    total_tokens = len(dataset) * max_length * num_epochs
    return total_tokens

def register_adapter_in_db(
    adapter_name: str,
    adapter_path: str,
    train_tokens: int,
) -> int:
    """Register the trained adapter in the database."""
    try:
        # Connect to the database
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Insert the adapter into the database
        cursor.execute(
            "INSERT INTO adapters (name, path, train_tokens) VALUES (?, ?, ?)",
            (adapter_name, adapter_path, train_tokens)
        )
        
        # Get the ID of the inserted adapter
        adapter_id = cursor.lastrowid
        
        # Commit the changes and close the connection
        conn.commit()
        conn.close()
        
        logger.info(f"Registered adapter in database with ID {adapter_id}")
        return adapter_id
    
    except sqlite3.Error as e:
        logger.error(f"Database error when registering adapter: {e}")
        raise

def main():
    """Main function to train a LoRA adapter from email data."""
    args = parse_args()
    
    # Find the latest data file if none is specified
    data_file = args.data_file or find_latest_data_file()
    if not data_file:
        logger.error("No email data file found. Please extract email data first.")
        sys.exit(1)
    
    logger.info(f"Using email data from {data_file}")
    
    # Load the email data
    email_data = load_email_data(data_file)
    
    # Generate adapter name if not provided
    if not args.adapter_name:
        subject = ""
        if "emailData" in email_data and "subject" in email_data["emailData"]:
            subject = email_data["emailData"]["subject"]
        elif "metadata" in email_data and "subject" in email_data["metadata"]:
            subject = email_data["metadata"]["subject"]

        if subject:
            # Clean up the subject to use as part of the adapter name
            clean_subject = "".join(c if c.isalnum() or c.isspace() else "_" for c in subject)
            clean_subject = clean_subject[:30]  # Limit length
            args.adapter_name = f"email_lora_{clean_subject}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        else:
            args.adapter_name = f"email_lora_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
    
    # Create output directory
    output_dir = args.output_dir if os.path.isabs(args.output_dir) else os.path.join(PROJECT_ROOT, args.output_dir)
    adapter_dir = os.path.join(output_dir, args.adapter_name)
    os.makedirs(adapter_dir, exist_ok=True)
    
    # Prepare the training data
    dataset = prepare_training_data(email_data)
    
    # Train the LoRA adapter
    train_tokens = train_lora_adapter(
        dataset=dataset,
        model_path=args.model_path,
        output_dir=adapter_dir,
        lora_r=args.lora_r,
        lora_alpha=args.lora_alpha,
        lora_dropout=args.lora_dropout,
        num_epochs=args.num_epochs,
        learning_rate=args.learning_rate,
        batch_size=args.batch_size,
        max_length=args.max_length,
    )
    
    # Register the adapter in the database
    adapter_path = os.path.relpath(adapter_dir, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    adapter_id = register_adapter_in_db(
        adapter_name=args.adapter_name,
        adapter_path=adapter_path,
        train_tokens=train_tokens,
    )
    
    logger.info(f"Successfully trained and registered LoRA adapter '{args.adapter_name}' (ID: {adapter_id})")
    logger.info(f"Adapter path: {adapter_path}")
    logger.info(f"Training tokens: {train_tokens}")

if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        logger.error(f"Error training LoRA adapter: {e}")
        sys.exit(1)
