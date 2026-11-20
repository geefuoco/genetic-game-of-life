/** Remembers across reloads whether the dock is docked to the side or floating. */

/** The part of `Storage` the dock needs. */
export interface DockStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const KEY = 'genetic-life:dock';
const DOCKED = 'side';
const FLOATING = 'floating';

/** The page's local storage, or `null` where the browser blocks it. */
export function browserStorage(): DockStorage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Whether the dock was last docked to the side; floating when unknown or storage is unavailable. */
export function loadDocked(storage: DockStorage | null): boolean {
  try {
    return storage?.getItem(KEY) === DOCKED;
  } catch {
    return false;
  }
}

/** Records the dock state, doing nothing when storage is unavailable. */
export function saveDocked(storage: DockStorage | null, docked: boolean): void {
  try {
    storage?.setItem(KEY, docked ? DOCKED : FLOATING);
  } catch {
    return;
  }
}
