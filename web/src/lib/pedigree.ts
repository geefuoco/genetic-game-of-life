/** The Pedigree graph in the inspector, and the memory budget of Recorded History. */

import type { Pedigree, PedigreeOrganism } from '../protocol.ts';

/** Bytes in a megabyte, the unit the settings show the Pedigree budget in. */
export const MB = 1_000_000;

/** The Pedigree budget the settings offer, in MB: the core's maximum, and the default on constrained devices. */
export const PEDIGREE_BUDGET_MB = { min: 10, max: 200, step: 10, constrained: 50 } as const;

/** Births back the inspector first asks for: the Organism and three rows of ancestors, four Generations of the Pedigree. */
export const PEDIGREE_DEPTH = 3;
/** Rows each press of "Show more ancestors" adds, and the most rows the inspector asks for. */
export const PEDIGREE_DEPTH_STEP = 2;
export const MAX_PEDIGREE_DEPTH = 15;

/** What the browser reports about the device. */
export interface Device {
  /** Approximate memory in GB, where the browser reports it. */
  deviceMemory?: number;
  /** Whether any pointer is fine, such as a mouse or trackpad. */
  finePointer: boolean;
  /** Whether any pointer is coarse, such as a finger. */
  coarsePointer: boolean;
}

/** Devices reporting less memory than this, in GB, count as low on memory. */
const LOW_MEMORY_GB = 4;

/** The Pedigree budget in bytes: 50 MB on a device that reports low memory or has only touch input, otherwise 200 MB. */
export function defaultPedigreeBudget(device: Device): number {
  const lowMemory = device.deviceMemory !== undefined && device.deviceMemory < LOW_MEMORY_GB;
  const touchOnly = device.coarsePointer && !device.finePointer;
  return (lowMemory || touchOnly ? PEDIGREE_BUDGET_MB.constrained : PEDIGREE_BUDGET_MB.max) * MB;
}

/** The device this page runs on. */
export function currentDevice(): Device {
  const memory = (navigator as Navigator & { deviceMemory?: number }).deviceMemory;
  return {
    ...(memory === undefined ? {} : { deviceMemory: memory }),
    finePointer: matchMedia('(any-pointer: fine)').matches,
    coarsePointer: matchMedia('(any-pointer: coarse)').matches,
  };
}

/** A budget as the settings show it, such as "50 MB". */
export function budgetLabel(bytes: number): string {
  return `${Math.round(bytes / MB)} MB`;
}

/** The name of the row of ancestors `depth` Births back, such as "Grandparents". */
export function rowLabel(depth: number): string {
  if (depth === 1) return 'Parents';
  if (depth === 2) return 'Grandparents';
  const greats = depth - 2;
  if (greats <= 2) return `Great-${'great-'.repeat(greats - 1)}grandparents`;
  return `${greats}× great-grandparents`;
}

function plural(count: number, one: string): string {
  return `${count} ${one}${count === 1 ? '' : 's'}`;
}

/** How an Organism in a Pedigree came to be, in words, such as "Born at gen 12 from 2 parents". */
export function describeAncestor(organism: PedigreeOrganism): string {
  const born = organism.birthGeneration === null ? null : `gen ${organism.birthGeneration.toLocaleString()}`;
  switch (organism.origin) {
    case 'placed':
      return `Painted or seeded at ${born ?? 'an unknown gen'}, no parents`;
    case 'beyond':
      return born ? `Born at ${born}, parents older than recorded history` : 'Parents older than recorded history';
    case 'unexplored':
      return `Born at ${born ?? 'an unknown gen'}, parents not shown yet`;
    case 'born': {
      const parents = organism.parents.length;
      if (parents === 0) return `Born at ${born} with no parents, both colour genes from the pool`;
      const pool = organism.fromAllelePool > 0 ? ` and ${plural(organism.fromAllelePool, 'colour gene')} from the pool` : '';
      return `Born at ${born} from ${plural(parents, 'parent')}${pool}`;
    }
  }
}

/** One Organism placed in the Pedigree graph. */
export interface PedigreeNode {
  /** Its index in {@link Pedigree.organisms}. */
  organism: number;
  /** Its row: its depth. */
  row: number;
  /** Its position across the width, between 0 and 1. */
  x: number;
}

/** A line from a child to one of its Parents, by index in {@link Pedigree.organisms}. */
export interface PedigreeEdge {
  child: number;
  parent: number;
}

export interface PedigreeLayout {
  rows: number;
  nodes: PedigreeNode[];
  edges: PedigreeEdge[];
}

/**
 * Lays a Pedigree out in rows by depth, each Organism once. Each row is
 * ordered by the mean position of its children in the rows below, so lines
 * cross as little as this simple ordering allows, and spread evenly.
 */
export function layoutPedigree(pedigree: Pedigree): PedigreeLayout {
  const { organisms } = pedigree;
  const edges: PedigreeEdge[] = organisms.flatMap((organism, child) => organism.parents.map(({ organism: parent }) => ({ child, parent })));
  const rows = organisms.reduce((deepest, organism) => Math.max(deepest, organism.depth), 0) + 1;
  const x = new Map<number, number>();
  const nodes: PedigreeNode[] = [];
  for (let row = 0; row < rows; row++) {
    const members = organisms.flatMap((organism, index) => (organism.depth === row ? [index] : []));
    const pull = (index: number): number => {
      const children = edges.filter((edge) => edge.parent === index).map((edge) => x.get(edge.child) ?? 0.5);
      return children.length === 0 ? 0.5 : children.reduce((sum, at) => sum + at, 0) / children.length;
    };
    const ordered = members.map((index) => ({ index, pull: pull(index) })).sort((a, b) => a.pull - b.pull || a.index - b.index);
    ordered.forEach(({ index }, slot) => {
      const at = (slot + 0.5) / ordered.length;
      x.set(index, at);
      nodes.push({ organism: index, row, x: at });
    });
  }
  return { rows, nodes, edges };
}
