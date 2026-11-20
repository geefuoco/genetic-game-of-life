import init, { WebWorld } from '../../wasm-pkg/genetic_life_wasm.js';
import type { Command, WorkerEvent } from '../protocol.ts';
import { Engine } from './engine.ts';

const send = (event: WorkerEvent, transfer: Transferable[]): void => {
  self.postMessage(event, { transfer });
};

const engineReady = init().then(
  () =>
    new Engine({
      createWorld: (soup) => {
        const world = new WebWorld(soup.seed, soup.width, soup.height, soup.density, soup.foundingAlleles, soup.rule, soup.mutation.rate, soup.mutation.magnitude);
        return {
          step: (generations) => world.step(generations),
          generation: () => world.generation(),
          population: () => world.population(),
          phenotypes: (out) => world.phenotypes(out),
          setRule: (rule) => world.setRule(rule),
          setMutation: (mutation) => world.setMutation(mutation.rate, mutation.magnitude),
          free: () => world.free(),
        };
      },
      clock: {
        now: () => performance.now(),
        setTimeout: (run, ms) => self.setTimeout(run, ms),
        clearTimeout: (id) => self.clearTimeout(id),
      },
      send,
    }),
);

engineReady.catch((error: unknown) => {
  send({ type: 'error', message: `The simulation could not load: ${String(error)}` }, []);
});

self.onmessage = (event: MessageEvent<Command>) => {
  void engineReady.then((engine) => engine.handle(event.data));
};
