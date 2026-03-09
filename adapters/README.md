# LoRA Adapters for Email Processing

This directory contains the remaining Python-only tooling for creating and using LoRA (Low-Rank Adaptation) adapters based on email data. The main application runtime is Swift + Rust; these scripts are kept only for training and adapter verification.

## Directory Structure

- `data/`: Contains JSON files with extracted email data
- `models/`: Contains trained LoRA adapter models
- `train_lora.py`: Python script for training LoRA adapters
- `create_lora.sh`: Shell script wrapper for easy adapter creation
- `test_lora.py`: Python script for testing LoRA adapters with the AI

## Workflow

1. **Extract Email Data**: Use the "Extract for LoRA" button in the Mail.app Integration app to extract data from the currently selected email. This creates a JSON file in the `data/` directory with both the canonical `metadata` / `content` / `training_examples` shape and compatibility keys for the legacy trainer.

2. **Train LoRA Adapter**: Run the `create_lora.sh` script to train a LoRA adapter based on the most recent email data:

   ```bash
   ./create_lora.sh
   ```

   This will:
   - Find the most recent email data file
   - Train a LoRA adapter using that data
   - Save the adapter to the `models/` directory
   - Register the adapter in the database

3. **Use the Adapter**: The adapter will be registered in the EmailBrain database and exposed in the web UI.

4. **Test the Adapter**: You can test if the AI can successfully use the adapter with the `test_lora.py` script:

   ```bash
   ./test_lora.py
   ```

   This will:
   - List all available adapters
   - Prompt you to select an adapter to test
   - Send a test prompt to the local EmailBrain backend using the selected adapter
   - Display the AI's response

   You can also specify an adapter ID and custom prompt:

   ```bash
   ./test_lora.py --adapter_id 1 --prompt "What was the subject of the last email?"
   ```

## Advanced Usage

For more control over the training process, you can use the `train_lora.py` script directly:

```bash
python3 train_lora.py --data_file adapters/data/your_data_file.json --adapter_name custom_name
```

### Available Options

- `--data_file`: Path to the JSON file containing email data
- `--model_path`: Path to the base model (default: data/models/phi-3-mini.gguf)
- `--output_dir`: Directory to save the trained adapter (default: adapters/models)
- `--adapter_name`: Name for the trained adapter
- `--lora_r`: LoRA attention dimension (default: 8)
- `--lora_alpha`: LoRA alpha parameter (default: 16)
- `--lora_dropout`: LoRA dropout probability (default: 0.05)
- `--num_epochs`: Number of training epochs (default: 3)
- `--learning_rate`: Learning rate (default: 3e-4)
- `--batch_size`: Training batch size (default: 4)
- `--max_length`: Maximum sequence length (default: 512)

The test harness reads `EMAILBRAIN_API_URL` from `.env` and targets `http://localhost:3901` by default.

## Requirements

- Python 3.8+
- PyTorch
- Transformers
- PEFT (Parameter-Efficient Fine-Tuning)
- Datasets

To install the required packages:

```bash
pip install torch transformers peft datasets
