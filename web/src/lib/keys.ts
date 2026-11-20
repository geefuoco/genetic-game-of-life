/** Keyboard shortcuts for the dock's controls. */

export type KeyAction = 'toggle-play' | 'step' | 'slower' | 'faster' | 'toggle-dock';

/** The parts of a key press that decide its action. */
export interface KeyPress {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
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
  if (press.editable || press.ctrlKey || press.metaKey || press.altKey) return null;
  return ACTIONS[press.key] ?? null;
}
