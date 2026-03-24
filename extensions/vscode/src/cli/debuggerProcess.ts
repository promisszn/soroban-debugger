import { spawn, ChildProcess } from 'child_process';
import { resolve } from 'path';

export interface DebuggerProcessConfig {
  contractPath: string;
  snapshotPath?: string;
  entrypoint?: string;
  args?: string[];
  trace?: boolean;
}

export class DebuggerProcess {
  private process: ChildProcess | null = null;
  private config: DebuggerProcessConfig;

  constructor(config: DebuggerProcessConfig) {
    this.config = config;
  }

  start(): Promise<void> {
    return new Promise((resolve, reject) => {
      const args = this.buildArgs();

      try {
        this.process = spawn('soroban-debugger', args, {
          stdio: ['pipe', 'pipe', 'pipe']
        });

        this.process.on('error', reject);
        this.process.on('spawn', resolve);
      } catch (error) {
        reject(error);
      }
    });
  }

  stop(): Promise<void> {
    return new Promise((resolve) => {
      if (!this.process) {
        resolve();
        return;
      }

      if (this.process.killed) {
        resolve();
        return;
      }

      this.process.once('exit', () => resolve());
      this.process.kill('SIGTERM');

      setTimeout(() => {
        if (this.process && !this.process.killed) {
          this.process.kill('SIGKILL');
        }
        resolve();
      }, 5000);
    });
  }

  sendCommand(command: any): Promise<any> {
    return new Promise((resolve, reject) => {
      if (!this.process || !this.process.stdin || !this.process.stdout) {
        reject(new Error('Debugger process not running'));
        return;
      }

      const input = JSON.stringify({
        id: Math.floor(Math.random() * 1000000),
        request: command
      }) + '\n';

      const listener = (data: Buffer) => {
        const lines = data.toString().split('\n');
        for (const line of lines) {
          if (!line.trim()) continue;
          try {
            const response = JSON.parse(line);
            if (response.response && response.response.type === command.type + 'Result') {
              this.process!.stdout!.removeListener('data', listener);
              resolve(response.response);
              return;
            }
          } catch (e) {
            // ignore non-json
          }
        }
      };

      this.process.stdout.on('data', listener);
      this.process.stdin.write(input);
      
      // Timeout after 10s
      setTimeout(() => {
        if (this.process && this.process.stdout) {
          this.process.stdout.removeListener('data', listener);
        }
        reject(new Error('Command timeout'));
      }, 10000);
    });
  }

  getInputStream() {
    return this.process?.stdin;
  }

  getOutputStream() {
    return this.process?.stdout;
  }

  getErrorStream() {
    return this.process?.stderr;
  }

  private buildArgs(): string[] {
    const args = ['debug'];

    args.push('--contract', this.config.contractPath);

    if (this.config.snapshotPath) {
      args.push('--snapshot', this.config.snapshotPath);
    }

    if (this.config.entrypoint) {
      args.push('--entrypoint', this.config.entrypoint);
    }

    if (this.config.args && this.config.args.length > 0) {
      args.push('--args', JSON.stringify(this.config.args));
    }

    if (this.config.trace) {
      args.push('--trace');
    }

    return args;
  }

  isRunning(): boolean {
    return this.process !== null && !this.process.killed;
  }
}
