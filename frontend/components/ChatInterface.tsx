'use client';

import { useEffect, useRef, useState } from 'react';
import { ChatRequest, Adapter } from '../lib/types';
import { apiClient } from '../lib/api';

interface ChatInterfaceProps {
  selectedAdapter?: Adapter;
  onAdapterInvalid?: () => void;
}

type ConversationMessage = {
  id: number;
  role: 'user' | 'assistant';
  content: string;
};

const MAX_PROMPT_CHARS = 4_000;
const MAX_CONTEXT_MESSAGES = 8;

export default function ChatInterface({
  selectedAdapter,
  onAdapterInvalid,
}: ChatInterfaceProps) {
  const [prompt, setPrompt] = useState('');
  const [messages, setMessages] = useState<ConversationMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const requestIdRef = useRef(0);

  useEffect(() => {
    return () => {
      abortControllerRef.current?.abort();
    };
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedPrompt = prompt.trim();
    if (!trimmedPrompt) return;

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    abortControllerRef.current?.abort();

    const abortController = new AbortController();
    abortControllerRef.current = abortController;

    try {
      setLoading(true);
      setError(null);
      setPrompt('');

      const chatRequest: ChatRequest = {
        prompt: buildChatPrompt(messages, trimmedPrompt),
        adapter_id: selectedAdapter?.id,
      };

      const chatResponse = await apiClient.sendChatMessage(chatRequest, abortController.signal);
      if (abortController.signal.aborted || requestId !== requestIdRef.current) {
        return;
      }

      const responseTexts = chatResponse.choices
        ?.map((choice) => choice.message?.content?.trim() ?? '')
        .filter(Boolean);

      setMessages((currentMessages) => {
        const nextMessages: ConversationMessage[] = [
          ...currentMessages,
          { id: requestId * 10, role: 'user', content: trimmedPrompt },
        ];

        if (responseTexts && responseTexts.length > 0) {
          responseTexts.forEach((content, index) => {
            nextMessages.push({
              id: requestId * 10 + index + 1,
              role: 'assistant',
              content,
            });
          });
        } else {
          nextMessages.push({
            id: requestId * 10 + 1,
            role: 'assistant',
            content: 'No response received from the AI model.',
          });
        }

        return nextMessages;
      });
    } catch (err) {
      if (abortController.signal.aborted || requestId !== requestIdRef.current) {
        return;
      }

      if (
        selectedAdapter &&
        err instanceof Error &&
        err.message === 'The requested item could not be found.'
      ) {
        onAdapterInvalid?.();
        setError('The selected adapter is no longer available. The selection was cleared.');
        return;
      }

      setPrompt(trimmedPrompt);
      setError(err instanceof Error ? err.message : 'Failed to send message');
      console.error('Error sending chat message:', err);
    } finally {
      if (!abortController.signal.aborted && requestId === requestIdRef.current) {
        setLoading(false);
      }
    }
  };

  const clearChat = () => {
    abortControllerRef.current?.abort();
    requestIdRef.current += 1;
    setPrompt('');
    setMessages([]);
    setError(null);
    setLoading(false);
  };

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-lg">
      <div className="px-4 py-5 sm:px-6 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg leading-6 font-medium text-gray-900">
              Chat with EmailBrain
            </h3>
            {selectedAdapter ? (
              <p className="mt-1 max-w-2xl text-sm text-gray-500">
                Using adapter: <span className="font-medium">{selectedAdapter.name}</span> via LM
                Studio
              </p>
            ) : (
              <p className="mt-1 max-w-2xl text-sm text-gray-500">
                No adapter selected - using the configured base-model provider
              </p>
            )}
          </div>
          <button
            type="button"
            onClick={clearChat}
            className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-2 rounded text-sm"
          >
            Clear
          </button>
        </div>
      </div>

      <div className="px-4 py-5 sm:p-6">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="prompt" className="block text-sm font-medium text-gray-700">
              Your message
            </label>
            <div className="mt-1">
              <textarea
                id="prompt"
                rows={3}
                className="shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md"
                placeholder="Ask a question about your emails or request analysis..."
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                disabled={loading}
                maxLength={MAX_PROMPT_CHARS}
              />
            </div>
            <p className="mt-2 text-xs text-gray-500">
              {prompt.length}/{MAX_PROMPT_CHARS} characters
            </p>
          </div>

          <div className="flex justify-end">
            <button
              type="submit"
              disabled={loading || !prompt.trim()}
              className="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? (
                <>
                  <div className="animate-spin -ml-1 mr-3 h-4 w-4 border-2 border-white border-t-transparent rounded-full"></div>
                  Sending...
                </>
              ) : (
                'Send Message'
              )}
            </button>
          </div>
        </form>

        {error && (
          <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-md">
            <div className="flex">
              <div className="ml-3">
                <h3 className="text-sm font-medium text-red-800">Error</h3>
                <div className="mt-2 text-sm text-red-700">
                  <p>{error}</p>
                </div>
              </div>
            </div>
          </div>
        )}

        {messages.length > 0 && (
          <div className="mt-4 space-y-4">
            <h4 className="text-sm font-medium text-gray-900">Conversation</h4>
            {messages.map((message) => (
              <div
                key={message.id}
                className={`rounded-md border p-4 ${
                  message.role === 'assistant'
                    ? 'border-gray-200 bg-gray-50'
                    : 'border-blue-200 bg-blue-50'
                }`}
              >
                <p className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-500">
                  {message.role === 'assistant' ? 'Assistant' : 'You'}
                </p>
                <pre className="max-h-96 overflow-auto whitespace-pre-wrap font-sans text-sm leading-relaxed text-gray-700">
                  {message.content}
                </pre>
              </div>
            ))}
          </div>
        )}

        {loading && (
          <div className="mt-4 rounded-md border border-blue-200 bg-blue-50 p-4 text-sm text-blue-900">
            Waiting for the local model runtime to respond...
          </div>
        )}
      </div>
    </div>
  );
}

function buildChatPrompt(messages: ConversationMessage[], prompt: string): string {
  const recentMessages = messages.slice(-MAX_CONTEXT_MESSAGES);
  if (recentMessages.length === 0) {
    return prompt;
  }

  const transcript = recentMessages
    .map((message) => `${message.role === 'assistant' ? 'Assistant' : 'User'}: ${message.content}`)
    .join('\n\n');

  return `${transcript}\n\nUser: ${prompt}`;
}
