/** Keeps the current World in the browser's IndexedDB, so a reload resumes it. */

import type { AutosavedWorld } from './persistence.ts';

/** How often the current World is autosaved, in ms. */
export const AUTOSAVE_MS = 10_000;

const DATABASE = 'genetic-life';
const STORE = 'autosave';
const KEY = 'world';

/** Where the current World is autosaved. */
export interface WorldStore {
  /** The autosaved World, or `null` if there is none or storage cannot be read. */
  load(): Promise<AutosavedWorld | null>;
  /** Autosaves a World over the last one, doing nothing when storage cannot be written. */
  save(world: AutosavedWorld): Promise<void>;
}

/** The browser's IndexedDB as a {@link WorldStore}. */
export function browserWorldStore(): WorldStore {
  let database: Promise<IDBDatabase> | null = null;
  const open = (): Promise<IDBDatabase> => {
    database ??= new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open(DATABASE, 1);
      request.onupgradeneeded = () => request.result.createObjectStore(STORE);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error ?? new Error('IndexedDB could not be opened'));
    });
    database.catch(() => {
      database = null;
    });
    return database;
  };
  return {
    async load() {
      try {
        const stored = await requested((await open()).transaction(STORE, 'readonly').objectStore(STORE).get(KEY));
        return asAutosavedWorld(stored);
      } catch {
        return null;
      }
    },
    async save(world) {
      try {
        await requested((await open()).transaction(STORE, 'readwrite').objectStore(STORE).put(world, KEY));
      } catch {
        return;
      }
    },
  };
}

function requested<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
  });
}

/** `stored` if it has the shape of a {@link AutosavedWorld}, otherwise `null`. */
function asAutosavedWorld(stored: unknown): AutosavedWorld | null {
  if (typeof stored !== 'object' || stored === null) return null;
  const { snapshot, link } = stored as Record<string, unknown>;
  if (!(snapshot instanceof Uint8Array) || (link !== null && typeof link !== 'string')) return null;
  return { snapshot, link };
}
