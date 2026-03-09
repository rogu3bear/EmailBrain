'use client';

import { useEffect, useRef, useState } from 'react';
import { Adapter } from '../lib/types';
import { apiClient } from '../lib/api';
import { formatShortDateTime } from '../lib/format';

interface AdapterListProps {
  onAdapterSelect?: (adapter: Adapter | null) => void;
  selectedAdapterId?: number;
  onSelectionInvalid?: () => void;
}

export default function AdapterList({
  onAdapterSelect,
  selectedAdapterId,
  onSelectionInvalid,
}: AdapterListProps) {
  const [adapters, setAdapters] = useState<Adapter[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const requestIdRef = useRef(0);
  const abortControllerRef = useRef<AbortController | null>(null);

  useEffect(() => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    abortControllerRef.current?.abort();

    const abortController = new AbortController();
    abortControllerRef.current = abortController;

    async function loadAdapters() {
      try {
        setLoading(true);
        const adapterData = await apiClient.getAdapters(abortController.signal);
        if (abortController.signal.aborted || requestId !== requestIdRef.current) {
          return;
        }

        setAdapters(adapterData);
        setError(null);
      } catch (err) {
        if (abortController.signal.aborted || requestId !== requestIdRef.current) {
          return;
        }

        setError(err instanceof Error ? err.message : 'Failed to fetch adapters');
        console.error('Error fetching adapters:', err);
      } finally {
        if (!abortController.signal.aborted && requestId === requestIdRef.current) {
          setLoading(false);
        }
      }
    }

    void loadAdapters();

    return () => {
      abortController.abort();
    };
  }, [refreshKey]);

  useEffect(() => {
    if (selectedAdapterId && !adapters.some((adapter) => adapter.id === selectedAdapterId)) {
      onSelectionInvalid?.();
    }
  }, [adapters, onSelectionInvalid, selectedAdapterId]);

  const refreshAdapters = () => {
    setRefreshKey((value) => value + 1);
  };

  const formatTokenCount = (tokens: number) => {
    if (tokens >= 1000000) {
      return `${(tokens / 1000000).toFixed(1)}M`;
    } else if (tokens >= 1000) {
      return `${(tokens / 1000).toFixed(1)}k`;
    }
    return tokens.toString();
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
            <h3 className="text-sm font-medium text-red-800">Error loading adapters</h3>
            <div className="mt-2 text-sm text-red-700">
              <p>{error}</p>
            </div>
            <div className="mt-4">
              <button
                type="button"
                onClick={refreshAdapters}
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

  if (adapters.length === 0) {
    return (
      <div className="text-center py-8">
        <h3 className="text-lg font-medium text-gray-900">No adapters found</h3>
        <p className="mt-2 text-gray-500">
          LoRA adapters you create will appear here.
        </p>
        <button
          type="button"
          onClick={refreshAdapters}
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
            LoRA Adapters ({adapters.length})
          </h3>
          <div className="flex items-center gap-2">
            {selectedAdapterId && (
              <button
                type="button"
                onClick={() => onAdapterSelect?.(null)}
                className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-2 rounded text-sm"
              >
                Clear selection
              </button>
            )}
            <button
              type="button"
              onClick={refreshAdapters}
              className="bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-2 rounded text-sm"
            >
              Refresh
            </button>
          </div>
        </div>
        <p className="mt-2 text-sm text-gray-500">
          Selecting an adapter routes chat through the adapter-capable LM Studio path.
        </p>
      </div>
      <ul className="divide-y divide-gray-200">
        {adapters.map((adapter) => (
          <li key={adapter.id}>
            <button
              type="button"
              className={`w-full px-4 py-4 text-left hover:bg-gray-50 focus:bg-gray-50 ${
                selectedAdapterId === adapter.id ? 'bg-blue-50 border-r-4 border-blue-500' : ''
              }`}
              onClick={() =>
                onAdapterSelect?.(selectedAdapterId === adapter.id ? null : adapter)
              }
              aria-pressed={selectedAdapterId === adapter.id}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center">
                    <p className="text-sm font-medium text-gray-900 truncate">
                      {adapter.name}
                    </p>
                    {selectedAdapterId === adapter.id && (
                      <span className="ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                        Selected
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-gray-500 truncate font-mono">
                    {adapter.path}
                  </p>
                  <div className="mt-2 flex items-center text-sm text-gray-500">
                    <span className="flex items-center">
                      <svg className="flex-shrink-0 mr-1.5 h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M6 2a1 1 0 00-1 1v1H4a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-1V3a1 1 0 10-2 0v1H7V3a1 1 0 00-1-1zm0 5a1 1 0 000 2h8a1 1 0 100-2H6z" clipRule="evenodd" />
                      </svg>
                      {formatTokenCount(adapter.train_tokens)} tokens
                    </span>
                    <span className="ml-4 flex items-center">
                      <svg className="flex-shrink-0 mr-1.5 h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clipRule="evenodd" />
                      </svg>
                      {formatShortDateTime(adapter.created_at)}
                    </span>
                  </div>
                </div>
              </div>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
