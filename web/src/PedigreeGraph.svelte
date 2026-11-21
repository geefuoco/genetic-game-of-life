<script lang="ts">
  import { describeAncestor, layoutPedigree, rowLabel, type PedigreeNode } from './lib/pedigree.ts';
  import type { Pedigree, PedigreeOrganism } from './protocol.ts';

  interface Props {
    pedigree: Pedigree;
  }

  let { pedigree }: Props = $props();

  /** Height of a row, the least space between Organisms in a row, and the margin around the graph, in CSS pixels. */
  const ROW = 36;
  const SPACING = 18;
  const MARGIN = 12;
  /** Width the graph fills before it scrolls sideways. */
  const MIN_WIDTH = 248;
  /** Length of the marks drawn below an Organism toward its unknown or unasked-for ancestors, and to the Allele Pool. */
  const TAIL = 15;

  const layout = $derived(layoutPedigree(pedigree));
  const widest = $derived(Math.max(1, ...Array.from({ length: layout.rows }, (_, row) => layout.nodes.filter((node) => node.row === row).length)));
  const width = $derived(Math.max(MIN_WIDTH, widest * SPACING + 2 * MARGIN));
  const height = $derived((layout.rows - 1) * ROW + 2 * MARGIN + TAIL + 4);
  const positions = $derived(new Map(layout.nodes.map((node) => [node.organism, point(node)])));
  const rows = $derived(Array.from({ length: layout.rows }, (_, row) => layout.nodes.filter((node) => node.row === row)));
  const shown = $derived(new Set(pedigree.organisms.map(markerOf)));

  type Marker = 'born' | 'placed' | 'beyond' | 'unexplored' | 'pool';

  function point(node: PedigreeNode): { x: number; y: number } {
    return { x: MARGIN + node.x * (width - 2 * MARGIN), y: MARGIN + node.row * ROW };
  }

  function at(index: number): { x: number; y: number } {
    return positions.get(index) ?? { x: 0, y: 0 };
  }

  function organism(index: number): PedigreeOrganism {
    const found = pedigree.organisms[index];
    if (!found) throw new Error(`the pedigree has no organism ${index}`);
    return found;
  }

  function markerOf(organism: PedigreeOrganism): Marker {
    return organism.origin === 'born' && organism.fromAllelePool > 0 ? 'pool' : organism.origin;
  }

  function edge(child: number, parent: number): string {
    const from = at(child);
    const to = at(parent);
    if (to.y === from.y) return `M${from.x},${from.y} Q${(from.x + to.x) / 2},${from.y + ROW * 0.8} ${to.x},${to.y}`;
    const middle = (from.y + to.y) / 2;
    return `M${from.x},${from.y} C${from.x},${middle} ${to.x},${middle} ${to.x},${to.y}`;
  }

  /** How far across the `draw`th of an Organism's Allele Pool draws is marked: beside the line to a lone Parent, or spread. */
  function poolMark(organism: PedigreeOrganism, draw: number): number {
    if (organism.parents.length > 0) return 9;
    return organism.fromAllelePool === 1 ? 0 : (draw - 0.5) * 8;
  }

  const LEGEND: readonly (readonly [Marker, string])[] = [
    ['born', 'Born from parents'],
    ['pool', 'Colour gene from the pool'],
    ['placed', 'Painted or seeded, no parents'],
    ['beyond', 'Older than recorded history'],
    ['unexplored', 'Parents not shown yet'],
  ];
</script>

{#snippet glyph(marker: Marker, x: number, y: number, root: boolean)}
  {@const r = root ? 6 : 4.5}
  {#if marker === 'placed'}
    <rect class="placed" x={x - r} y={y - r} width={r * 2} height={r * 2} rx="2" />
  {:else if marker === 'beyond'}
    <line class="tail beyond" x1={x} y1={y + r} x2={x} y2={y + TAIL} />
    <circle class="beyond" cx={x} cy={y} {r} />
  {:else if marker === 'unexplored'}
    <circle class="unexplored" cx={x} cy={y} {r} />
  {:else}
    <circle class="born" cx={x} cy={y} {r} />
  {/if}
{/snippet}

<div class="graph">
  <svg {width} {height} viewBox="0 0 {width} {height}">
    <g class="edges" aria-hidden="true">
      {#each layout.edges as { child, parent } (`${child}-${parent}`)}
        <path d={edge(child, parent)} />
      {/each}
    </g>
    {#each rows as row, depth (depth)}
      <g role="group" aria-label={depth === 0 ? 'This cell' : rowLabel(depth)}>
        {#each row as node (node.organism)}
          {@const { x, y } = at(node.organism)}
          {@const ancestor = organism(node.organism)}
          {@const label = describeAncestor(ancestor)}
          <g role="img" aria-label={label} class:root={depth === 0}>
            <title>{label}</title>
            {#each { length: ancestor.origin === 'born' ? ancestor.fromAllelePool : 0 }, draw (draw)}
              {@const dx = poolMark(ancestor, draw)}
              <line class="tail pool" x1={x} y1={y + 4} x2={x + dx} y2={y + TAIL - 3} />
              <rect class="pool" x={x + dx - 3} y={y + TAIL - 3} width="6" height="6" transform="rotate(45 {x + dx} {y + TAIL})" />
            {/each}
            {@render glyph(ancestor.origin, x, y, depth === 0)}
          </g>
        {/each}
      </g>
    {/each}
  </svg>
</div>
<ul class="legend">
  {#each LEGEND.filter(([marker]) => shown.has(marker)) as [marker, text] (marker)}
    <li>
      <svg width="16" height="20" viewBox="0 0 16 20" aria-hidden="true">
        {#if marker === 'pool'}
          <line class="tail pool" x1="8" y1="5" x2="8" y2="12" />
          <rect class="pool" x="5" y="12" width="6" height="6" transform="rotate(45 8 15)" />
          {@render glyph('born', 8, 5, false)}
        {:else}
          {@render glyph(marker, 8, 7, false)}
        {/if}
      </svg>
      {text}
    </li>
  {/each}
</ul>

<style>
  .graph {
    overflow-x: auto;
  }

  svg {
    display: block;
  }

  .edges path {
    fill: none;
    stroke: rgb(255 255 255 / 22%);
    stroke-width: 1.2;
  }

  .born {
    fill: #dfe3ec;
  }

  .placed {
    fill: #c9a86a;
  }

  .beyond {
    fill: var(--glass);
    stroke: var(--muted);
    stroke-width: 1.4;
    stroke-dasharray: 2 2;
  }

  .unexplored {
    fill: var(--glass);
    stroke: #dfe3ec;
    stroke-width: 1.4;
  }

  .tail {
    stroke-width: 1.2;
  }

  .tail.beyond {
    fill: none;
    stroke-dasharray: 1.5 2.5;
  }

  .tail.pool {
    stroke: #7fb3d5;
    stroke-dasharray: 1 2;
  }

  rect.pool {
    fill: #7fb3d5;
  }

  .root circle,
  .root rect:not(.pool) {
    stroke: #fff;
    stroke-width: 1.6;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 12px;
    padding: 0;
    margin-top: 4px;
    font-size: 11px;
    color: var(--muted);
    list-style: none;
  }

  .legend li {
    display: flex;
    gap: 4px;
    align-items: center;
  }
</style>
