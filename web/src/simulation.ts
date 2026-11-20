import type { Command, Frame, WorkerEvent } from './protocol.ts';

export interface SimulationHandlers {
  frame(frame: Frame): void;
  error(message: string): void;
}

/** The page's side of the simulation worker. */
export class Simulation {
  private readonly worker: Worker;

  constructor(handlers: SimulationHandlers) {
    this.worker = new Worker(new URL('./worker/worker.ts', import.meta.url), { type: 'module' });
    this.worker.onmessage = (event: MessageEvent<WorkerEvent>) => {
      const message = event.data;
      if (message.type === 'frame') handlers.frame(message);
      else handlers.error(message.message);
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
