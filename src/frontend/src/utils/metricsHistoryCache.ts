import type { HistorySample } from '../api';

const DB_NAME = 'mydns-dashboard';
const DB_VERSION = 1;
const STORE_NAME = 'metrics-history';
const RETENTION_MS = 24 * 60 * 60 * 1000;

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (!('indexedDB' in window)) {
      reject(new Error('IndexedDB is unavailable'));
      return;
    }

    const request = window.indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(STORE_NAME)) {
        database.createObjectStore(STORE_NAME, { keyPath: 'timestamp' });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('Failed to open metrics cache'));
  });
}

export function mergeHistorySamples(
  current: HistorySample[],
  incoming: HistorySample[],
  now = Date.now(),
): HistorySample[] {
  const byTimestamp = new Map<string, HistorySample>();

  for (const sample of current) {
    byTimestamp.set(sample.timestamp, sample);
  }
  for (const sample of incoming) {
    byTimestamp.set(sample.timestamp, sample);
  }

  return [...byTimestamp.values()]
    .filter(sample => Date.parse(sample.timestamp) >= now - RETENTION_MS)
    .sort((a, b) => Date.parse(a.timestamp) - Date.parse(b.timestamp));
}

export async function loadHistoryCache(): Promise<HistorySample[]> {
  try {
    const database = await openDatabase();
    const samples = await new Promise<HistorySample[]>((resolve, reject) => {
      const transaction = database.transaction(STORE_NAME, 'readonly');
      const request = transaction.objectStore(STORE_NAME).getAll();
      request.onsuccess = () => resolve(request.result as HistorySample[]);
      request.onerror = () => reject(request.error ?? new Error('Failed to read metrics cache'));
    });
    database.close();

    return mergeHistorySamples([], samples);
  } catch {
    return [];
  }
}

export async function saveHistoryCache(samples: HistorySample[]): Promise<void> {
  try {
    const database = await openDatabase();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(STORE_NAME, 'readwrite');
      const store = transaction.objectStore(STORE_NAME);
      const cutoff = Date.now() - RETENTION_MS;

      for (const sample of samples) {
        if (Date.parse(sample.timestamp) >= cutoff) {
          store.put(sample);
        }
      }

      const request = store.openCursor();
      request.onsuccess = () => {
        const cursor = request.result;
        if (!cursor) {
          return;
        }
        if (Date.parse(String(cursor.key)) < cutoff) {
          cursor.delete();
        }
        cursor.continue();
      };
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error ?? new Error('Failed to write metrics cache'));
      transaction.onabort = () => reject(transaction.error ?? new Error('Metrics cache transaction aborted'));
    });
    database.close();
  } catch {
    // The server remains authoritative; an unavailable browser cache should
    // never make the dashboard unusable.
  }
}
