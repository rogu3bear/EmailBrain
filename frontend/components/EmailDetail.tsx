'use client';

import { useCallback, useEffect, useState } from 'react';
import { Email } from '../lib/types';
import { apiClient } from '../lib/api';

interface EmailDetailProps {
  emailId: number;
  onBack?: () => void;
}

export default function EmailDetail({ emailId, onBack }: EmailDetailProps) {
  const [email, setEmail] = useState<Email | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchEmail = useCallback(async () => {
    try {
      setLoading(true);
      const emailData = await apiClient.getEmail(emailId);
      setEmail(emailData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch email');
      console.error('Error fetching email:', err);
    } finally {
      setLoading(false);
    }
  }, [emailId]);

  useEffect(() => {
    void fetchEmail();
  }, [emailId, fetchEmail]);

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 bg-red-50 border border-red-200 rounded-md">
        <div className="flex">
          <div className="ml-3">
            <h3 className="text-sm font-medium text-red-800">Error loading email</h3>
            <div className="mt-2 text-sm text-red-700">
              <p>{error}</p>
            </div>
            <div className="mt-4 space-x-2">
              <button
                onClick={fetchEmail}
                className="bg-red-100 hover:bg-red-200 text-red-800 px-3 py-2 rounded text-sm"
              >
                Try again
              </button>
              {onBack && (
                <button
                  onClick={onBack}
                  className="bg-gray-100 hover:bg-gray-200 text-gray-800 px-3 py-2 rounded text-sm"
                >
                  Back to list
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!email) {
    return (
      <div className="text-center py-8">
        <h3 className="text-lg font-medium text-gray-900">Email not found</h3>
        <p className="mt-2 text-gray-500">
          The requested email could not be found.
        </p>
        {onBack && (
          <button
            onClick={onBack}
            className="mt-4 bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
          >
            Back to list
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-lg">
      <div className="px-4 py-5 sm:px-6 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div className="flex-1 min-w-0">
            <h3 className="text-lg leading-6 font-medium text-gray-900">
              {email.subject || 'No Subject'}
            </h3>
            <p className="mt-1 max-w-2xl text-sm text-gray-500">
              Email details and content
            </p>
          </div>
          {onBack && (
            <button
              onClick={onBack}
              className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-2 rounded text-sm"
            >
              ← Back to list
            </button>
          )}
        </div>
      </div>
      
      <div className="px-4 py-5 sm:p-6">
        <dl className="grid grid-cols-1 gap-x-4 gap-y-6 sm:grid-cols-2">
          <div>
            <dt className="text-sm font-medium text-gray-500">From</dt>
            <dd className="mt-1 text-sm text-gray-900">{email.sender}</dd>
          </div>
          
          {email.recipients && (
            <div>
              <dt className="text-sm font-medium text-gray-500">To</dt>
              <dd className="mt-1 text-sm text-gray-900">{email.recipients}</dd>
            </div>
          )}
          
          <div>
            <dt className="text-sm font-medium text-gray-500">Date</dt>
            <dd className="mt-1 text-sm text-gray-900">{formatDate(email.date)}</dd>
          </div>
          
          {email.thread_id && (
            <div>
              <dt className="text-sm font-medium text-gray-500">Thread ID</dt>
              <dd className="mt-1 text-sm text-gray-900 font-mono text-xs">{email.thread_id}</dd>
            </div>
          )}
        </dl>
        
        {email.body && (
          <div className="mt-6">
            <dt className="text-sm font-medium text-gray-500 mb-2">Content</dt>
            <dd className="mt-1 text-sm text-gray-900">
              <div className="bg-gray-50 p-4 rounded-md">
                <pre className="whitespace-pre-wrap font-sans text-sm leading-relaxed">
                  {email.body}
                </pre>
              </div>
            </dd>
          </div>
        )}
      </div>
    </div>
  );
}
