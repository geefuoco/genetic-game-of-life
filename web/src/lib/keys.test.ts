import { describe, expect, it } from 'vitest';
import { keyAction, type KeyPress } from './keys.ts';

const press = (key: string, extra: Partial<KeyPress> = {}): KeyPress => ({
  key,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
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

  it('undoes with Ctrl+Z or Cmd+Z', () => {
    expect(keyAction(press('z', { ctrlKey: true }))).toBe('undo');
    expect(keyAction(press('z', { metaKey: true }))).toBe('undo');
  });

  it('redoes with Ctrl+Shift+Z, Cmd+Shift+Z or Ctrl+Y', () => {
    expect(keyAction(press('Z', { ctrlKey: true, shiftKey: true }))).toBe('redo');
    expect(keyAction(press('z', { metaKey: true, shiftKey: true }))).toBe('redo');
    expect(keyAction(press('y', { ctrlKey: true }))).toBe('redo');
  });

  it('leaves undo and redo to an input that has focus', () => {
    expect(keyAction(press('z', { ctrlKey: true, editable: true }))).toBeNull();
    expect(keyAction(press('y', { ctrlKey: true, editable: true }))).toBeNull();
  });

  it('ignores Z without Ctrl or Cmd, and with Alt', () => {
    expect(keyAction(press('z'))).toBeNull();
    expect(keyAction(press('z', { ctrlKey: true, altKey: true }))).toBeNull();
  });

  it('ignores other keys', () => {
    expect(keyAction(press('x'))).toBeNull();
  });
});
