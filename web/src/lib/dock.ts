/** Remembers across reloads whether the dock is docked to the side or floating. */

/** The part of `Storage` the dock needs. */
export interface DockStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const KEY = 'genetic-life:dock';
const DOCKED = 'side';
const FLOATING = 'floating';

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
