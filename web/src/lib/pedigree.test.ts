import { describe, expect, it } from 'vitest';
import type { Pedigree, PedigreeOrganism } from '../protocol.ts';
import { budgetLabel, defaultPedigreeBudget, describeAncestor, layoutPedigree, rowLabel } from './pedigree.ts';

/** A megabyte, written out so the expected budgets do not depend on the module's own unit. */
const MB = 1_000_000;

describe('the default Pedigree budget', () => {
  it('is 200 MB on a device with a mouse or trackpad and enough memory', () => {
    expect(defaultPedigreeBudget({ deviceMemory: 8, finePointer: true, coarsePointer: false })).toBe(200 * MB);
    expect(defaultPedigreeBudget({ finePointer: true, coarsePointer: false })).toBe(200 * MB);
    expect(defaultPedigreeBudget({ deviceMemory: 8, finePointer: true, coarsePointer: true })).toBe(200 * MB);
  });

  it('is 50 MB on a device that reports low memory', () => {
    expect(defaultPedigreeBudget({ deviceMemory: 2, finePointer: true, coarsePointer: false })).toBe(50 * MB);
  });

  it('is 50 MB on a touch-only device', () => {
    expect(defaultPedigreeBudget({ deviceMemory: 8, finePointer: false, coarsePointer: true })).toBe(50 * MB);
  });
});

describe('labels', () => {
  it('show a budget in whole megabytes', () => {
    expect(budgetLabel(50 * MB)).toBe('50 MB');
  });

  it('name each row of ancestors by how far back it is', () => {
    expect([1, 2, 3, 4, 6].map(rowLabel)).toEqual([
      'Parents',
      'Grandparents',
      'Great-grandparents',
      'Great-great-grandparents',
      '4× great-grandparents',
    ]);
  });
});

function organism(overrides: Partial<PedigreeOrganism>): PedigreeOrganism {
  return { x: 0, y: 0, birthGeneration: 5, depth: 1, origin: 'born', parents: [], fromAllelePool: 0, ...overrides };
}

describe('describing an ancestor', () => {
  it('tells Parents and the Allele Pool apart from having no Parents', () => {
    const parents = [
      { organism: 1, allele: 0 as const },
      { organism: 2, allele: 1 as const },
    ];
    expect(describeAncestor(organism({ birthGeneration: 12, parents }))).toBe('Born at gen 12 from 2 parents');
    expect(describeAncestor(organism({ parents: parents.slice(1), fromAllelePool: 1 }))).toBe(
      'Born at gen 5 from 1 parent and 1 colour gene from the pool',
    );
    expect(describeAncestor(organism({ fromAllelePool: 2 }))).toBe('Born at gen 5 with no parents, both colour genes from the pool');
    expect(describeAncestor(organism({ origin: 'placed', birthGeneration: 0 }))).toBe('Painted or seeded at gen 0, no parents');
  });

  it('marks ancestry older than recorded history as unknown rather than absent', () => {
    expect(describeAncestor(organism({ origin: 'beyond', birthGeneration: null }))).toBe('Parents older than recorded history');
    expect(describeAncestor(organism({ origin: 'beyond', birthGeneration: 3 }))).toBe('Born at gen 3, parents older than recorded history');
    expect(describeAncestor(organism({ origin: 'unexplored' }))).toBe('Born at gen 5, parents not shown yet');
  });
});

describe('laying out a Pedigree', () => {
  /**
   * The root's Parents are A (1) and B (2). A's own Parent C (4) comes after
   * B's own Parent D (3) in the list, and A and B share E (5).
   */
  const pedigree: Pedigree = {
    organisms: [
      organism({ depth: 0, parents: [{ organism: 1, allele: 0 }, { organism: 2, allele: 0 }] }),
      organism({ parents: [{ organism: 4, allele: 0 }, { organism: 5, allele: 0 }] }),
      organism({ parents: [{ organism: 3, allele: 1 }, { organism: 5, allele: 1 }] }),
      organism({ depth: 2, origin: 'placed' }),
      organism({ depth: 2, origin: 'placed' }),
      organism({ depth: 2, origin: 'beyond' }),
    ],
  };

  it('puts each Organism once in the row of its depth, with an edge to each child', () => {
    const layout = layoutPedigree(pedigree);
    expect(layout.rows).toBe(3);
    expect(layout.nodes.map((node) => node.organism).sort()).toEqual([0, 1, 2, 3, 4, 5]);
    expect(layout.nodes.find((node) => node.organism === 5)?.row).toBe(2);
    expect(layout.edges.filter((edge) => edge.parent === 5).map((edge) => edge.child).sort()).toEqual([1, 2]);
    expect(layout.edges).toHaveLength(6);
  });

  it('orders each row under its children and spreads it across the width', () => {
    const x = (organism: number): number => layoutPedigree(pedigree).nodes.find((node) => node.organism === organism)?.x ?? NaN;
    expect(x(0)).toBe(0.5);
    expect(x(1)).toBeLessThan(x(2));
    expect(x(4)).toBeLessThan(x(5));
    expect(x(5)).toBeLessThan(x(3));
    for (const node of layoutPedigree(pedigree).nodes) {
      expect(node.x).toBeGreaterThan(0);
      expect(node.x).toBeLessThan(1);
    }
  });
});
