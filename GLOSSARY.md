# Genetic Game of Life

A cellular automaton faithful to Conway's Game of Life, where every live cell also carries heritable genetic material that is passed on at birth, so ancestry becomes visible as color.

## Language

### The world

**World**:
A fixed-size grid of cells whose edges wrap around on both axes (a torus).
_Avoid_: Board, grid, universe, map

**Cell**:
One position in the World, either live or dead at any moment; a live Cell carries a Genotype.
_Avoid_: Pixel, tile, square

**Neighbour**:
One of the eight Cells surrounding a given Cell, wrapping across World edges.
_Avoid_: Adjacent cell

**Generation**:
One complete, simultaneous application of the Rule to every Cell in the World.
_Avoid_: Tick, step, frame, turn

### Rules

**Rule**:
The pair of neighbour counts that decide Birth and Survival, written in B/S notation (e.g. B3/S23 for Conway's Life).
_Avoid_: Ruleset, ruleString

**Rule Preset**:
A named, well-known Rule offered for quick selection (e.g. Conway's Life, HighLife).
_Avoid_: Template

**Birth**:
A dead Cell becoming live in the next Generation because its live Neighbour count is in the Rule's birth set.
_Avoid_: Spawn, creation

**Survival**:
A live Cell staying live in the next Generation because its live Neighbour count is in the Rule's survival set.
_Avoid_: Persistence

### Genetics

**Gene**:
A heritable trait slot; each Gene has a pair of Alleles in every live Cell.
_Avoid_: Trait, chromosome

**Pigment**:
The Gene whose Alleles are hues and which determines a Cell's color.
_Avoid_: Color gene

**Allele**:
One variant of a Gene, defined by its hue and its Dominance Level.
_Avoid_: Gene (when one variant is meant), color

**Genotype**:
The pair of Alleles a live Cell carries for each Gene.
_Avoid_: DNA, genome

**Phenotype**:
The visible color of a live Cell, derived from its Genotype.
_Avoid_: Display color, appearance

**Dominance Level**:
An Allele's rank: a higher-ranked Allele masks a lower-ranked one, and equal ranks blend (Incomplete Dominance).
_Avoid_: Strength, priority

**Incomplete Dominance**:
When two Alleles of equal Dominance Level both contribute to the Phenotype as a blend.
_Avoid_: Codominance, mixing

**Carrier**:
A live Cell holding an Allele that its Phenotype does not show.
_Avoid_: Hidden cell

**Parent**:
One of up to two live Neighbours chosen at random at a Birth, each passing on one Allele per Gene.
_Avoid_: Ancestor (for direct parents), source

**Mutation**:
A random change to an Allele as it is passed on, producing a new Allele with a shifted hue and a random Dominance Level.
_Avoid_: Variation, drift

**Allele Pool**:
Every Allele carried by the live Cells, counted once per copy; a Birth with fewer than two Parents draws its missing Alleles from it at random.
_Avoid_: Gene pool, random allele

**Founding Allele**:
An Allele present at Generation 0, with no Allele it descends from.
_Avoid_: Root allele, original

**Allele Tree**:
The record of which Allele each Allele mutated from, and in which Generation.
_Avoid_: Phylogeny, gene tree

**Pedigree**:
The record of a Cell's Parents, their Parents, and so on back in time.
_Avoid_: Family tree, ancestry (when the Allele Tree is meant)

**Recorded History**:
The span of recent Generations for which Pedigrees are still retained; ancestry older than this is no longer known.
_Avoid_: Ancestry window, buffer

### Seeding and sharing

**Seed**:
The number that fixes every random choice in a run, so the same starting state, Rule, and settings always produce the same run.
_Avoid_: Random state

**Soup**:
A starting World filled at random with live Cells.
_Avoid_: Random fill, noise

**Pattern**:
A named, known arrangement of live Cells (e.g. glider, Gosper gun) that can be stamped into the World.
_Avoid_: Shape, prefab, template

**Share Link**:
Text encoding the Rule, settings and Seed of a Soup World, from which anyone can start the same run.
_Avoid_: Permalink, share code

**Snapshot**:
A complete, exportable record of a World's state at one Generation, sufficient to resume the run exactly.
_Avoid_: Save, dump, state file
