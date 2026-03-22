import { DebugSession } from '@vscode/debugadapter';
import { SorobanDebugSession } from './dap/adapter';

DebugSession.run(SorobanDebugSession);
