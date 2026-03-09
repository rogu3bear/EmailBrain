export interface Email {
  id: number;
  subject: string;
  sender: string;
  body: string;
  date: string;
  recipients?: string | null;
  thread_id?: string | null;
}

export interface Adapter {
  id: number;
  name: string;
  path: string;
  train_tokens: number;
  created_at: string;
}

export interface ChatRequest {
  prompt: string;
  adapter_id?: number;
}

export interface ChatMessage {
  role: string;
  content: string;
}

export interface ChatChoice {
  index: number;
  message: ChatMessage;
  finish_reason?: string;
}

export interface ChatResponse {
  id?: string;
  object?: string;
  created?: number;
  model?: string;
  choices: ChatChoice[];
}
