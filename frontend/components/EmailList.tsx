'use client';

import { useState, useEffect } from 'react';
import { Email } from '../lib/types';
import { apiClient } from '../lib/api';

interface EmailListProps {
  onEmailSelect?: (email: Email) => void;
}

export default function EmailList({ onEmailSelect }: EmailListProps) {
  const [emails, setEmails] = useState<Email[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchEmails();
  }, []);

  const fetchEmails = async () => {
    try {
      setLoading(true);
      const emailData = await apiClient.getEmails();
      setEmails(emailData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch emails');
      console.error('Error fetching emails:', err);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
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
            <h3 className="text-sm font-medium text-red-800">Error loading emails</h3>
            <div className="mt-2 text-sm text-red-700">
              <p>{error}</p>
            </div>
            <div className="mt-4">
              <button
                onClick={fetchEmails}
                className="bg-red-100 hover:bg-red-200 text-red-800 px-3 py-2 rounded text-sm"
              >
                Try again
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (emails.length === 0) {
    return (
      <div className="text-center py-8">
        <h3 className="text-lg font-medium text-gray-900">No emails found</h3>
        <p className="mt-2 text-gray-500">
          Emails from your Swift Mail app will appear here.
        </p>
        <button
          onClick={fetchEmails}
          className="mt-4 bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded"
        >
          Refresh
        </button>
      </div>
    );
  }

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-md">
      <div className="px-4 py-3 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <h3 className="text-lg leading-6 font-medium text-gray-900">
            Emails ({emails.length})
          </h3>
          <button
            onClick={fetchEmails}
            className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-2 rounded text-sm"
          >
            Refresh
          </button>
        </div>
      </div>
      <ul className="divide-y divide-gray-200">
        {emails.map((email) => (
          <li key={email.id}>
            <div
              className="px-4 py-4 hover:bg-gray-50 cursor-pointer"
              onClick={() => onEmailSelect?.(email)}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-gray-900 truncate">
                    {email.subject || 'No Subject'}
                  </p>
                  <p className="text-sm text-gray-500 truncate">
                    From: {email.sender}
                  </p>
                  {email.recipients && (
                    <p className="text-sm text-gray-500 truncate">
                      To: {email.recipients}
                    </p>
                  )}
                </div>
                <div className="flex-shrink-0 text-sm text-gray-500">
                  {formatDate(email.date)}
                </div>
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}