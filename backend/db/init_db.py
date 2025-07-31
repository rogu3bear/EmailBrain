#!/usr/bin/env python3
"""
Database initialization script for EmailBrain.
Creates the SQLite database with the required schema.
"""

import os
import sqlite3
import sys
from pathlib import Path

def init_database():
    """Initialize the database with the schema"""
    # Get the directory containing this script
    script_dir = Path(__file__).parent
    db_path = script_dir / "mail.db"
    schema_path = script_dir / "schema.sql"
    
    # Check if schema file exists
    if not schema_path.exists():
        print(f"Error: Schema file not found at {schema_path}")
        return False
    
    # Read the schema
    with open(schema_path, 'r') as f:
        schema_sql = f.read()
    
    # Create or connect to the database
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Execute the schema
        cursor.executescript(schema_sql)
        conn.commit()
        
        print(f"Database initialized successfully at {db_path}")
        
        # Verify tables were created
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table';")
        tables = cursor.fetchall()
        print(f"Created tables: {', '.join([table[0] for table in tables])}")
        
        conn.close()
        return True
        
    except sqlite3.Error as e:
        print(f"Database error: {e}")
        return False
    except Exception as e:
        print(f"Unexpected error: {e}")
        return False

def main():
    """Main function"""
    print("EmailBrain Database Initialization")
    print("=" * 40)
    
    if init_database():
        print("Database initialization completed successfully!")
        sys.exit(0)
    else:
        print("Database initialization failed!")
        sys.exit(1)

if __name__ == "__main__":
    main()