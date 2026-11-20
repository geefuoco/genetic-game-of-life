import type { Command, Frame, WorkerEvent } from './protocol.ts';

export interface SimulationHandlers {
  frame(frame: Frame): void;
  /** The Rule now running, in canonical notation. */
  rule(rule: string): void;
  /** A Rule the core refused, with its explanation. */
  ruleRejected(rule: string, message: string): void;
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
        case 'rule':
          handlers.rule(message.rule);
          break;
        case 'rule-rejected':
          handlers.ruleRejected(message.rule, message.message);
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

  /** Sends a command, transferring the buffer of a `release`. */
  send(command: Command): void {
    if (command.type === 'release') this.worker.postMessage(command, [command.phenotypes.buffer]);
    else this.worker.postMessage(command);
  }

  dispose(): void {
    this.worker.terminate();
  }
}
