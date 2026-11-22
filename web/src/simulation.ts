import { transferable, type Census, type Command, type Frame, type Pedigree, type WorkerEvent } from './protocol.ts';

export interface SimulationHandlers {
  frame(frame: Frame): void;
  /** The census of the World, while it is watched. */
  census(census: Census): void;
  /** The Rule now running, in canonical notation. */
  rule(rule: string): void;
  /** A Rule the core refused, with its explanation. */
  ruleRejected(rejected: { rule: string; message: string }): void;
  /** The inspected Organism's Pedigree, `depth` Births back, or `null` if nothing is inspected. */
  pedigree(pedigree: Pedigree | null, depth: number): void;
  error(message: string): void;
}

/** The page's side of the simulation worker. */
export class Simulation {
  private readonly worker: Worker;

  constructor(handlers: SimulationHandlers) {
    this.worker = new Worker(new URL('./worker/worker.ts', import.meta.url), { type: 'module' });
    this.worker.onmessage = (event: MessageEvent<WorkerEvent>) => {
      const message = event.data;
      switch (message.type) {
        case 'frame':
          handlers.frame(message);
          break;
        case 'census':
          handlers.census(message);
          break;
        case 'rule':
          handlers.rule(message.rule);
          break;
        case 'rule-rejected':
          handlers.ruleRejected({ rule: message.rule, message: message.message });
          break;
        case 'pedigree':
          handlers.pedigree(message.pedigree, message.depth);
          break;
        case 'error':
          handlers.error(message.message);
          break;
      }
    };
    this.worker.onerror = (event) => {
      handlers.error(`The simulation stopped: ${event.message}`);
    };
  }

  /** Sends a command, transferring the buffers of a `release`. */
  send(command: Command): void {
    if (command.type === 'release') this.worker.postMessage(command, transferable(command));
    else this.worker.postMessage(command);
  }

  dispose(): void {
    this.worker.terminate();
  }
}
