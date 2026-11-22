/** How the page opens a World, shares it by link, and keeps it as a Snapshot. */

/** A World autosaved in the browser: its Snapshot, and the Share Link of the Soup run it is on, if any. */
export interface AutosavedWorld {
  snapshot: Uint8Array;
  link: string | null;
}

/** One way to open the first World: a Share Link, the autosaved World, or a fresh Soup. */
export type Opening = { from: 'link'; link: string } | { from: 'autosave'; autosaved: AutosavedWorld } | { from: 'soup' };

/**
 * The ways to open the first World, to try in order until one is accepted: the Share Link in the address's fragment,
 * then the autosaved World, then a fresh Soup, which always opens.
 */
export function openingPlan(fragment: string, autosaved: AutosavedWorld | null): Opening[] {
  const link = linkInFragment(fragment);
  const plan: Opening[] = [];
  if (link !== '') plan.push({ from: 'link', link });
  if (autosaved) plan.push({ from: 'autosave', autosaved });
  plan.push({ from: 'soup' });
  return plan;
}

/** The Share Link an address's fragment holds, such as `location.hash`, or `''` if it has none. */
export function linkInFragment(fragment: string): string {
  const link = fragment.replace(/^#/, '');
  try {
    return decodeURIComponent(link);
  } catch {
    return link;
  }
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
