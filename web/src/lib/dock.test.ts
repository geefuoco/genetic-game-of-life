import { describe, expect, it } from 'vitest';
import { loadDocked, saveDocked, type DockStorage } from './dock.ts';

function memoryStorage(): DockStorage {
  const items = new Map<string, string>();
  return {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => void items.set(key, value),
  };
}

const broken: DockStorage = {
  getItem: () => {
    throw new Error('storage is disabled');
  },
  setItem: () => {
    throw new Error('storage is disabled');
  },
};

describe('dock persistence', () => {
  it('floats by default', () => {
    expect(loadDocked(memoryStorage())).toBe(false);
  });

  it('remembers docking to the side', () => {
    const storage = memoryStorage();
    saveDocked(storage, true);
    expect(loadDocked(storage)).toBe(true);
  });

  it('remembers floating again', () => {
    const storage = memoryStorage();
    saveDocked(storage, true);
    saveDocked(storage, false);
    expect(loadDocked(storage)).toBe(false);
  });

  it('floats when storage holds something else', () => {
    const storage = memoryStorage();
    storage.setItem('genetic-life:dock', 'sideways');
    expect(loadDocked(storage)).toBe(false);
  });

  it('floats, and saving does nothing, when storage is unavailable', () => {
    expect(loadDocked(broken)).toBe(false);
    expect(() => saveDocked(broken, true)).not.toThrow();
  });
});
