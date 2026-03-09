import type { Adapter, ChatRequest, ChatResponse, Email } from './types';

const API_BASE_URL =
  process.env.NEXT_PUBLIC_EMAILBRAIN_API_URL?.replace(/\/$/, '') ??
  'http://localhost:3901';

type ErrorPayload = {
  detail?: string;
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    cache: 'no-store',
  });

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`;

    try {
      const payload = (await response.json()) as ErrorPayload;
      if (payload.detail) {
        message = payload.detail;
      }
    } catch {
      // Fall back to the status-derived message when the error payload is absent.
    }

    throw new Error(message);
  }

  return (await response.json()) as T;
}

export const apiClient = {
  async getEmails(): Promise<Email[]> {
    const payload = await request<{ emails: Email[] }>('/api/v1/emails');
    return payload.emails;
  },

  async getEmail(emailId: number): Promise<Email> {
    const payload = await request<{ email: Email }>(`/api/v1/emails/${emailId}`);
    return payload.email;
  },

  async getAdapters(): Promise<Adapter[]> {
    const payload = await request<{ adapters: Adapter[] }>('/api/v1/adapters');
    return payload.adapters;
  },

  async sendChatMessage(chatRequest: ChatRequest): Promise<ChatResponse> {
    return request<ChatResponse>('/api/v1/chat', {
      method: 'POST',
      body: JSON.stringify(chatRequest),
    });
  },
};
