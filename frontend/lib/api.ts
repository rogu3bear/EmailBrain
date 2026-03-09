import type { Adapter, ChatRequest, ChatResponse, Email } from './types';

export const API_BASE_URL =
  process.env.NEXT_PUBLIC_EMAILBRAIN_API_URL?.replace(/\/$/, '') ??
  'http://localhost:3901';

type ErrorPayload = {
  detail?: string;
};

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  if (init?.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers,
    cache: 'no-store',
  });

  if (!response.ok) {
    let detail = '';

    try {
      const contentType = response.headers.get('content-type') ?? '';
      if (contentType.includes('application/json')) {
        const payload = (await response.json()) as ErrorPayload;
        detail = payload.detail?.trim() ?? '';
      } else {
        detail = (await response.text()).trim();
      }
    } catch {
      // Fall back to the status-derived message when the error payload is absent.
    }

    throw new Error(buildErrorMessage(response.status, detail));
  }

  if (response.status === 204) {
    return null as T;
  }

  const contentType = response.headers.get('content-type') ?? '';
  if (contentType.includes('application/json')) {
    return (await response.json()) as T;
  }

  const text = (await response.text()).trim();
  if (!text) {
    return null as T;
  }

  try {
    return JSON.parse(text) as T;
  } catch {
    throw new Error('The server returned an unexpected response format.');
  }
}

function buildErrorMessage(status: number, detail: string): string {
  switch (status) {
    case 400:
      return detail || 'The request was invalid.';
    case 404:
      return 'The requested item could not be found.';
    case 408:
    case 504:
      return 'The request timed out. Please try again.';
    case 503:
      return 'The local EmailBrain services are unavailable. Check the backend and model runtimes.';
    default:
      if (status >= 500) {
        return 'The local EmailBrain services returned an error. Check the backend and model runtimes.';
      }
      return detail || `Request failed with status ${status}`;
  }
}

export const apiClient = {
  async getEmails(signal?: AbortSignal): Promise<Email[]> {
    const payload = await request<{ emails: Email[] }>('/api/v1/emails', { signal });
    return payload.emails;
  },

  async getEmail(emailId: number, signal?: AbortSignal): Promise<Email> {
    const payload = await request<{ email: Email }>(`/api/v1/emails/${emailId}`, { signal });
    return payload.email;
  },

  async getAdapters(signal?: AbortSignal): Promise<Adapter[]> {
    const payload = await request<{ adapters: Adapter[] }>('/api/v1/adapters', { signal });
    return payload.adapters;
  },

  async sendChatMessage(chatRequest: ChatRequest, signal?: AbortSignal): Promise<ChatResponse> {
    return request<ChatResponse>('/api/v1/chat', {
      method: 'POST',
      body: JSON.stringify(chatRequest),
      signal,
    });
  },
};
