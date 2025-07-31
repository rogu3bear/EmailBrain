-- v1  (SQLite & DuckDB identical)
CREATE TABLE emails(
  id INTEGER PRIMARY KEY,
  thread_id TEXT,
  sender TEXT,
  recipients TEXT,
  subject TEXT,
  date TIMESTAMP,
  body TEXT
);
CREATE TABLE adapters(
  id INTEGER PRIMARY KEY,
  name TEXT,
  path TEXT,
  train_tokens INT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE insights(
  id INTEGER PRIMARY KEY,
  email_id INT,
  type TEXT,
  payload JSON
);
CREATE TABLE logs(
  id INTEGER PRIMARY KEY,
  ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  prompt TEXT,
  response TEXT,
  tokens_in INT,
  tokens_out INT,
  adapter_id INT
);
