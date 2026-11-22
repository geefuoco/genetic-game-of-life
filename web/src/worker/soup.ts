import { WebSoup } from '../../wasm-pkg/genetic_life_wasm.js';
import type { SoupSettings } from '../protocol.ts';

/** The core's Share Link of a Soup; throws the core's explanation if its Rule or Mutation settings are invalid. */
export function shareLinkOf(soup: SoupSettings): string {
  const web = new WebSoup(soup.seed, soup.rule, soup.width, soup.height, soup.density, soup.foundingAlleles);
  try {
    web.setMutation(soup.mutation.rate, soup.mutation.magnitude);
    return web.shareLink();
  } finally {
    web.free();
  }
}

/** The Soup a Share Link describes, decoded by the core; throws the core's explanation if it cannot be decoded. */
export function soupOfShareLink(link: string): SoupSettings {
  const web = WebSoup.fromShareLink(link);
  try {
    return {
      seed: web.seed(),
      rule: web.rule(),
      width: web.width(),
      height: web.height(),
      density: web.density(),
      foundingAlleles: web.foundingAlleles(),
      mutation: { rate: web.mutationRate(), magnitude: web.mutationMagnitude() },
    };
  } finally {
    web.free();
  }
}
