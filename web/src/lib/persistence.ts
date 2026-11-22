/** How the page opens a World, shares it by link, and keeps it as a Snapshot. */

/** A World autosaved in the browser: its Snapshot, and the Share Link of the Soup run it is on, if any. */
export interface SavedWorld {
  snapshot: Uint8Array;
  link: string | null;
}

/** One way to open the first World: a Share Link, the autosaved World, or a fresh Soup. */
export type Opening = { from: 'link'; link: string } | { from: 'autosave'; saved: SavedWorld } | { from: 'soup' };

/**
 * The ways to open the first World, to try in order until one is accepted: the Share Link in the address's fragment,
 * then the autosaved World, then a fresh Soup, which always opens.
 */
export function openingPlan(fragment: string, saved: SavedWorld | null): Opening[] {
  const link = fragment.replace(/^#/, '');
  const plan: Opening[] = [];
  if (link !== '') plan.push({ from: 'link', link });
  if (saved) plan.push({ from: 'autosave', saved });
  plan.push({ from: 'soup' });
  return plan;
}

/** The address of the page at `href` that opens the run a Share Link describes. */
export function shareUrl(href: string, link: string): string {
  const url = new URL(href);
  url.hash = link;
  return url.href;
}

/** The name a downloaded Snapshot of Generation `generation` is saved under. */
export function snapshotFileName(generation: number): string {
  return `genetic-life-gen-${generation}.ggls`;
}

/** What the visitor is told when the core refuses a Share Link or Snapshot, with `kept` saying whether a World was running and stays. */
export function rejectionNotice(source: 'link' | 'snapshot', message: string, kept: boolean): string {
  const what = source === 'link' ? "That share link can't be opened" : "That snapshot can't be imported";
  return `${what}: ${message}.${kept ? ' Your world is unchanged.' : ''}`;
}
