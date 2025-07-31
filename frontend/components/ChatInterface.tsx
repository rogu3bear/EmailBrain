'use client';

import { useState } from 'react';
import { ChatRequest, ChatResponse, Adapter } from '../lib/types';
import { apiClient } from '../lib/api';

interface ChatInterfaceProps {
  selectedAdapter?: Adapter;
}

export default function ChatInterface({ selectedAdapter }: ChatInterfaceProps) {
  const [prompt, setPrompt] = useState('');
  const [response, setResponse] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!prompt.trim()) return;

    try {
      setLoading(true);
      setError(null);
      
      const chatRequest: ChatRequest = {
        prompt: prompt.trim(),
        adapter_id: selectedAdapter?.id
      };

      const chatResponse = await apiClient.sendChatMessage(chatRequest);
      
      if (chatResponse.choices && chatResponse.choices.length > 0) {
        setResponse(chatResponse.choices[0].message.content);
      } else {
        setResponse('No response received from the AI model.');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
      console.error('Error sending chat message:', err);
    } finally {
      setLoading(false);
    }
  };

  const clearChat = () => {
    setPrompt('');
    setResponse('');
    setError(null);
  };

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-lg">
      <div className="px-4 py-5 sm:px-6 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg leading-6 font-medium text-gray-900">
              Chat with EmailBrain
            </h3>
            {selectedAdapter && (
              <p className="mt-1 max-w-2xl text-sm text-gray-500">
                Using adapter: <span className="font-medium">{selectedAdapter.name}</span>
              </p>
            )}
            {!selectedAdapter && (
              <p className="mt-1 max-w-2xl text-sm text-gray-500">
                No adapter selected - using base model
              </p>
            )}
          </div>
          <button
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
              />
            </div>
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

        {response && (
          <div className="mt-4 p-4 bg-gray-50 border border-gray-200 rounded-md">
            <h4 className="text-sm font-medium text-gray-900 mb-2">AI Response:</h4>
            <div className="text-sm text-gray-700">
              <pre className="whitespace-pre-wrap font-sans leading-relaxed">
                {response}
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}