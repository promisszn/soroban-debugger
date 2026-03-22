import {
  DebugSession,
  InitializedEvent,
  StoppedEvent,
  ExitedEvent} from '@vscode/debugadapter';
import { DebugProtocol } from '@vscode/debugprotocol';
import * as readline from 'readline';
import { DebuggerProcess, DebuggerProcessConfig } from '../cli/debuggerProcess';
import { DebuggerState, Variable } from './protocol';
import { LogOutputEvent, LogLevel } from '@vscode/debugadapter/lib/logger';

type LaunchRequestArgs = DebugProtocol.LaunchRequestArguments & DebuggerProcessConfig;

export class SorobanDebugSession extends DebugSession {
  private debuggerProcess: DebuggerProcess | null = null;
  private state: DebuggerState = {
    isRunning: false,
    isPaused: false,
    breakpoints: new Map(),
    callStack: [],
    variables: []
  };
  private variableHandles = new Map<number, any>();
  private nextVarHandle = 1;
  private threadId = 1;
  private outputReaders: readline.Interface[] = [];
  private hasExecuted = false;

  protected initializeRequest(
    response: DebugProtocol.InitializeResponse,
    args: DebugProtocol.InitializeRequestArguments
  ): void {
    response.body = response.body || {};
    response.body.supportsConfigurationDoneRequest = true;
    response.body.supportsEvaluateForHovers = true;
    response.body.supportsSetVariable = false;
    response.body.supportsSetExpression = false;
    response.body.supportsConditionalBreakpoints = false;
    response.body.supportsHitConditionalBreakpoints = false;
    response.body.supportsLogPoints = false;

    this.sendResponse(response);
    this.sendEvent(new InitializedEvent());
  }

  protected async launchRequest(
    response: DebugProtocol.LaunchResponse,
    args: LaunchRequestArgs
  ): Promise<void> {
    try {
      this.debuggerProcess = new DebuggerProcess({
        contractPath: args.contractPath,
        snapshotPath: args.snapshotPath,
        entrypoint: args.entrypoint || 'main',
        args: args.args || [],
        trace: args.trace || false,
        binaryPath: args.binaryPath,
        port: args.port,
        token: args.token
      });

      await this.debuggerProcess.start();
      this.state.isRunning = true;
      this.state.isPaused = false;
      this.hasExecuted = false;
      this.variableHandles.clear();
      this.nextVarHandle = 1;

      this.attachProcessListeners();
      this.sendResponse(response);
    } catch (error) {
      this.sendErrorResponse(response, {
        id: 1001,
        format: `Failed to launch debugger: ${error}`,
        showUser: true
      });
    }
  }

  protected async setBreakpointsRequest(
    response: DebugProtocol.SetBreakpointsResponse,
    args: DebugProtocol.SetBreakpointsArguments
  ): Promise<void> {
    const source = args.source.path || args.source.name || '';
    const breakpoints = args.breakpoints || [];

    this.state.breakpoints.set(source,
      breakpoints.map((bp, idx) => ({
        source,
        line: bp.line,
        column: bp.column
      }))
    );

    response.body = {
      breakpoints: breakpoints.map(bp => ({
        verified: false,
        line: bp.line,
        column: bp.column,
        source: args.source,
        message: 'Source breakpoints are not supported by the current Soroban debug backend'
      }))
    };

    this.sendResponse(response);
  }

  protected async stackTraceRequest(
    response: DebugProtocol.StackTraceResponse,
    args: DebugProtocol.StackTraceArguments
  ): Promise<void> {
    const stackFrames = this.state.callStack || [];

    response.body = {
      stackFrames: stackFrames.slice(0, 50).map(frame => ({
        id: frame.id,
        name: frame.name,
        source: {
          name: frame.source,
          path: frame.source
        },
        line: frame.line,
        column: frame.column,
        instructionPointerReference: frame.instructionPointerReference
      }))
    };

    this.sendResponse(response);
  }

  protected async scopesRequest(
    response: DebugProtocol.ScopesResponse,
    args: DebugProtocol.ScopesArguments
  ): Promise<void> {
    const scopes: DebugProtocol.Scope[] = [];

    if (this.state.variables && this.state.variables.length > 0) {
      const variablesRef = this.nextVarHandle++;
      this.variableHandles.set(variablesRef, this.state.variables);

      scopes.push({
        name: 'Storage',
        variablesReference: variablesRef,
        expensive: false
      });
    }

    response.body = { scopes };
    this.sendResponse(response);
  }

  protected async variablesRequest(
    response: DebugProtocol.VariablesResponse,
    args: DebugProtocol.VariablesArguments
  ): Promise<void> {
    const variables = this.variableHandles.get(args.variablesReference) || [];

    response.body = {
      variables: variables.map((v: Variable) => ({
        name: v.name,
        value: v.value,
        type: v.type,
        variablesReference: v.variablesReference || 0
      }))
    };

    this.sendResponse(response);
  }

  protected async continueRequest(
    response: DebugProtocol.ContinueResponse,
    args: DebugProtocol.ContinueArguments
  ): Promise<void> {
    try {
      if (!this.debuggerProcess) {
        throw new Error('Debugger process is not running');
      }

      response.body = { allThreadsContinued: true };
      this.sendResponse(response);

      if (!this.hasExecuted) {
        await this.runExecution('step');
        return;
      }

      this.sendEvent(new ExitedEvent(0));
      await this.stop();
    } catch (error) {
      this.sendErrorResponse(response, {
        id: 1002,
        format: `Continue failed: ${error}`,
        showUser: true
      });
    }
  }

  protected async nextRequest(
    response: DebugProtocol.NextResponse,
    args: DebugProtocol.NextArguments
  ): Promise<void> {
    await this.stepOnce(response, 'next');
  }

  protected async stepInRequest(
    response: DebugProtocol.StepInResponse,
    args: DebugProtocol.StepInArguments
  ): Promise<void> {
    await this.stepOnce(response, 'step in');
  }

  protected async stepOutRequest(
    response: DebugProtocol.StepOutResponse,
    args: DebugProtocol.StepOutArguments
  ): Promise<void> {
    await this.stepOnce(response, 'step out');
  }

  protected async threadRequest(
    response: DebugProtocol.ThreadsResponse
  ): Promise<void> {
    response.body = {
      threads: [{
        id: this.threadId,
        name: 'Main Thread'
      }]
    };
    this.sendResponse(response);
  }

  protected async configurationDoneRequest(
    response: DebugProtocol.ConfigurationDoneResponse,
    args: DebugProtocol.ConfigurationDoneArguments
  ): Promise<void> {
    if (this.debuggerProcess) {
      await this.refreshState();
      this.state.isPaused = true;
      this.sendEvent(new StoppedEvent('entry', this.threadId));
    }

    this.sendResponse(response);
  }

  protected async disconnectRequest(
    response: DebugProtocol.DisconnectResponse,
    args: DebugProtocol.DisconnectArguments
  ): Promise<void> {
    await this.stop();
    this.sendResponse(response);
  }

  private attachProcessListeners(): void {
    if (!this.debuggerProcess) return;

    const stdout = this.debuggerProcess.getOutputStream();
    if (stdout) {
      const reader = readline.createInterface({
        input: stdout,
        crlfDelay: Infinity
      });

      reader.on('line', (line: string) => {
        this.sendEvent(new LogOutputEvent(line + '\n', LogLevel.Log));
      });
      this.outputReaders.push(reader);
    }

    const stderr = this.debuggerProcess.getErrorStream();
    if (stderr) {
      const reader = readline.createInterface({
        input: stderr,
        crlfDelay: Infinity
      });

      reader.on('line', (line: string) => {
        this.sendEvent(new LogOutputEvent(line + '\n', LogLevel.Error));
      });
      this.outputReaders.push(reader);
    }
  }

  private async runExecution(reason: DebugProtocol.StoppedEvent['reason']): Promise<void> {
    if (!this.debuggerProcess) {
      throw new Error('Debugger process is not running');
    }

    await this.debuggerProcess.execute();
    this.hasExecuted = true;
    await this.refreshState();
    this.state.isPaused = true;
    this.sendEvent(new StoppedEvent(reason, this.threadId));
  }

  private async stepOnce(
    response:
      | DebugProtocol.NextResponse
      | DebugProtocol.StepInResponse
      | DebugProtocol.StepOutResponse,
    label: string
  ): Promise<void> {
    try {
      if (!this.debuggerProcess) {
        throw new Error('Debugger process is not running');
      }

      this.sendResponse(response);

      if (!this.hasExecuted) {
        await this.runExecution('step');
        return;
      }

      await this.debuggerProcess.step();
      await this.refreshState();
      this.state.isPaused = true;
      this.sendEvent(new StoppedEvent('step', this.threadId));
    } catch (error) {
      this.sendEvent(new LogOutputEvent(`${label} failed: ${error}\n`, LogLevel.Error));
    }
  }

  private async refreshState(): Promise<void> {
    if (!this.debuggerProcess) {
      return;
    }

    const [inspection, storage] = await Promise.all([
      this.debuggerProcess.inspect(),
      this.debuggerProcess.getStorage()
    ]);

    this.state.callStack = inspection.callStack.map((frame, index) => ({
      id: index + 1,
      name: frame,
      source: frame,
      line: 1,
      column: 1
    }));
    this.state.variables = this.storageToVariables(storage);
  }

  private storageToVariables(storage: Record<string, unknown>): Variable[] {
    return Object.entries(storage)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, value]) => ({
        name,
        value: typeof value === 'string' ? value : JSON.stringify(value),
        type: Array.isArray(value) ? 'array' : typeof value,
        variablesReference: 0
      }));
  }

  public async stop(): Promise<void> {
    for (const reader of this.outputReaders) {
      reader.close();
    }
    this.outputReaders = [];

    if (this.debuggerProcess) {
      await this.debuggerProcess.stop();
      this.debuggerProcess = null;
    }

    this.state.isRunning = false;
    this.state.isPaused = false;
    this.state.callStack = [];
    this.state.variables = [];
    this.hasExecuted = false;
  }
}
