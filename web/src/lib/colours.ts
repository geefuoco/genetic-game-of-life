/** Colour names for hues, the UI copy GLOSSARY.md maps to Alleles and Phenotypes. */

/** Each name's centre hue, in degrees; a hue takes the name whose centre is nearest around the wheel. */
const NAMES: readonly (readonly [number, string])[] = [
  [0, 'rose'],
  [29, 'red'],
  [50, 'orange'],
  [75, 'amber'],
  [100, 'yellow'],
  [123, 'lime'],
  [145, 'green'],
  [165, 'jade'],
  [186, 'teal'],
  [207, 'cyan'],
  [232, 'sky'],
  [262, 'blue'],
  [285, 'indigo'],
  [305, 'violet'],
  [324, 'purple'],
  [344, 'pink'],
];

/** The colour name of a hue in degrees, such as "orange". */
export function colourName(hue: number): string {
  let best = NAMES[0]?.[1] ?? '';
  let nearest = Infinity;
  for (const [centre, name] of NAMES) {
    const apart = Math.abs(hue - centre);
    const distance = Math.min(apart, 360 - apart);
    if (distance < nearest) {
      nearest = distance;
      best = name;
    }
  }
  return best;
}

/** `text` with its first letter capitalised. */
export function capitalised(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
