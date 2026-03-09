'use client';

import { useCallback, useEffect, useState } from 'react';
import EmailList from '../components/EmailList';
import EmailDetail from '../components/EmailDetail';
import AdapterList from '../components/AdapterList';
import ChatInterface from '../components/ChatInterface';
import { Email, Adapter } from '../lib/types';
import { API_BASE_URL } from '../lib/api';
import { formatEndpointLabel } from '../lib/format';

export default function HomePage() {
  const [selectedEmail, setSelectedEmail] = useState<Email | null>(null);
  const [selectedAdapter, setSelectedAdapter] = useState<Adapter | null>(null);
  const [activeTab, setActiveTab] = useState<'emails' | 'adapters' | 'chat'>('emails');
  const [frontendOrigin, setFrontendOrigin] = useState('Current origin');

  const handleEmailSelect = useCallback((email: Email) => {
    setSelectedEmail(email);
  }, []);

  const handleAdapterSelect = useCallback((adapter: Adapter | null) => {
    setSelectedAdapter(adapter);
  }, []);

  const handleBackToList = useCallback(() => {
    setSelectedEmail(null);
  }, []);

  const clearSelectedAdapter = useCallback(() => {
    setSelectedAdapter(null);
  }, []);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      setFrontendOrigin(window.location.origin);
    }
  }, []);

  const handleTabChange = (nextTab: 'emails' | 'adapters' | 'chat') => {
    setActiveTab(nextTab);
    if (nextTab !== 'emails') {
      setSelectedEmail(null);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-6">
            <div className="flex items-center">
              <h1 className="text-3xl font-bold text-gray-900">EmailBrain</h1>
              <span className="ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                v1.0.0
              </span>
            </div>
            <div className="flex items-center space-x-4">
              <div className="text-sm text-gray-500">
                {selectedAdapter ? (
                  <span>
                    Adapter: <strong>{selectedAdapter.name}</strong>
                  </span>
                ) : (
                  <span>No adapter selected</span>
                )}
              </div>
              {selectedAdapter && (
                <button
                  type="button"
                  onClick={clearSelectedAdapter}
                  className="rounded bg-gray-100 px-3 py-2 text-sm text-gray-700 hover:bg-gray-200"
                >
                  Clear adapter
                </button>
              )}
            </div>
          </div>
        </div>
      </header>

      {/* Navigation Tabs */}
      <nav className="bg-white border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex space-x-8">
            <button
              type="button"
              onClick={() => handleTabChange('emails')}
              className={`py-4 px-1 border-b-2 font-medium text-sm ${
                activeTab === 'emails'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              Emails
            </button>
            <button
              type="button"
              onClick={() => handleTabChange('adapters')}
              className={`py-4 px-1 border-b-2 font-medium text-sm ${
                activeTab === 'adapters'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              Adapters
            </button>
            <button
              type="button"
              onClick={() => handleTabChange('chat')}
              className={`py-4 px-1 border-b-2 font-medium text-sm ${
                activeTab === 'chat'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              Chat
            </button>
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {activeTab === 'emails' && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <div>
              <EmailList
                onEmailSelect={handleEmailSelect}
                selectedEmailId={selectedEmail?.id}
                onSelectionInvalid={handleBackToList}
              />
            </div>
            <div>
              {selectedEmail ? (
                <EmailDetail
                  emailId={selectedEmail.id}
                  onBack={handleBackToList}
                />
              ) : (
                <div className="bg-white shadow overflow-hidden sm:rounded-lg p-6">
                  <div className="text-center">
                    <h3 className="text-lg font-medium text-gray-900">Select an email</h3>
                    <p className="mt-2 text-gray-500">
                      Choose an email from the list to view its details.
                    </p>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {activeTab === 'adapters' && (
          <div className="max-w-4xl">
            <AdapterList
              onAdapterSelect={handleAdapterSelect}
              selectedAdapterId={selectedAdapter?.id}
              onSelectionInvalid={clearSelectedAdapter}
            />
          </div>
        )}

        {activeTab === 'chat' && (
          <div className="max-w-4xl">
            <ChatInterface
              selectedAdapter={selectedAdapter || undefined}
              onAdapterInvalid={clearSelectedAdapter}
            />
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="bg-white border-t border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex justify-between items-center text-sm text-gray-500">
            <p>EmailBrain - AI-powered email analysis</p>
            <div className="flex items-center space-x-4">
              <span>Backend: {formatEndpointLabel(API_BASE_URL)}</span>
              <span>Frontend: {formatEndpointLabel(frontendOrigin)}</span>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
