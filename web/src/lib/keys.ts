/** Keyboard shortcuts for the dock's controls. */

export type KeyAction = 'toggle-play' | 'step' | 'slower' | 'faster' | 'toggle-dock' | 'undo' | 'redo';

/** The parts of a key press that decide its action. */
export interface KeyPress {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  /** Whether focus is in an input, where keys type rather than act. */
  editable: boolean;
}

const ACTIONS: Record<string, KeyAction> = {
  ' ': 'toggle-play',
  '.': 'step',
  '[': 'slower',
  ']': 'faster',
  d: 'toggle-dock',
  D: 'toggle-dock',
};

/** The action a key press triggers, if any. */
export function keyAction(press: KeyPress): KeyAction | null {
  if (press.editable || press.altKey) return null;
  if (press.ctrlKey || press.metaKey) return commandAction(press);
  return ACTIONS[press.key] ?? null;
}

/** Undo and redo, the only Ctrl or Cmd shortcuts the page takes over. */
function commandAction(press: KeyPress): KeyAction | null {
  const key = press.key.toLowerCase();
  if (key === 'z') return press.shiftKey ? 'redo' : 'undo';
  if (key === 'y' && press.ctrlKey) return 'redo';
  return null;
}
