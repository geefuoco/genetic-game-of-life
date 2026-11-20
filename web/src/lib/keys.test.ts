import { describe, expect, it } from 'vitest';
import { keyAction, type KeyPress } from './keys.ts';

const press = (key: string, extra: Partial<KeyPress> = {}): KeyPress => ({
  key,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  editable: false,
  ...extra,
});

describe('keys', () => {
  it('plays or pauses with Space', () => {
    expect(keyAction(press(' '))).toBe('toggle-play');
  });

  it('steps with a full stop', () => {
    expect(keyAction(press('.'))).toBe('step');
  });

  it('changes speed with square brackets', () => {
    expect(keyAction(press('['))).toBe('slower');
    expect(keyAction(press(']'))).toBe('faster');
  });

  it('docks or floats with D in either case', () => {
    expect(keyAction(press('d'))).toBe('toggle-dock');
    expect(keyAction(press('D'))).toBe('toggle-dock');
  });

  it('does nothing while focus is in an input', () => {
    expect(keyAction(press(' ', { editable: true }))).toBeNull();
    expect(keyAction(press('d', { editable: true }))).toBeNull();
  });

  it('leaves browser shortcuts alone', () => {
    expect(keyAction(press('d', { ctrlKey: true }))).toBeNull();
    expect(keyAction(press('d', { metaKey: true }))).toBeNull();
    expect(keyAction(press('.', { altKey: true }))).toBeNull();
  });

  it('ignores other keys', () => {
    expect(keyAction(press('x'))).toBeNull();
  });
});
