#!/bin/bash
# Script to create a LoRA adapter from the most recent email data

set -e

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Check if Python and required packages are installed
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is required but not installed."
    exit 1
fi

# Check if any email data files exist
DATA_FILES=("$SCRIPT_DIR/data"/*.json)
if [ ${#DATA_FILES[@]} -eq 0 ] || [ ! -f "${DATA_FILES[0]}" ]; then
    echo "Error: No email data files found in $SCRIPT_DIR/data."
    echo "Please extract email data first using the 'Extract for LoRA' button in the Mail.app Integration app."
    exit 1
fi

# Find the most recent email data file
LATEST_DATA_FILE=$(ls -t "$SCRIPT_DIR/data"/*.json | head -n 1)
echo "Using most recent email data file: $LATEST_DATA_FILE"

# Run the LoRA training script
echo "Starting LoRA adapter training..."
python3 "$SCRIPT_DIR/train_lora.py" --data_file "$LATEST_DATA_FILE"

echo "LoRA adapter training completed successfully!"
echo "You can now use the adapter in the Mail.app Integration app."