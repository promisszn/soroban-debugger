import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import { DebuggerProcess } from '../cli/debuggerProcess';
import { resolveSourceBreakpoints } from '../dap/sourceBreakpoints';

async function main(): Promise<void> {
  const extensionRoot = process.cwd();
  const repoRoot = path.resolve(extensionRoot, '..', '..');

  const emittedFiles = [
    path.join(extensionRoot, 'dist', 'extension.js'),
    path.join(extensionRoot, 'dist', 'debugAdapter.js'),
    path.join(extensionRoot, 'dist', 'cli', 'debuggerProcess.js')
  ];

  for (const file of emittedFiles) {
    assert.ok(fs.existsSync(file), `Missing compiled artifact: ${file}`);
  }

  const binaryPath = process.env.SOROBAN_DEBUG_BIN
    || path.join(repoRoot, 'target', 'debug', process.platform === 'win32' ? 'soroban-debug.exe' : 'soroban-debug');

  if (!fs.existsSync(binaryPath)) {
    console.log(`Skipping debugger smoke test because the CLI binary was not found at ${binaryPath}`);
    return;
  }

  const contractPath = path.join(repoRoot, 'tests', 'fixtures', 'wasm', 'counter.wasm');
  assert.ok(fs.existsSync(contractPath), `Missing fixture WASM: ${contractPath}`);

  const debuggerProcess = new DebuggerProcess({
    binaryPath,
    contractPath,
    entrypoint: 'increment',
    args: []
  });

  await debuggerProcess.start();
  await debuggerProcess.ping();

  const sourcePath = path.join(repoRoot, 'tests', 'fixtures', 'contracts', 'counter', 'src', 'lib.rs');
  const exportedFunctions = await debuggerProcess.getContractFunctions();
  const resolvedBreakpoints = resolveSourceBreakpoints(sourcePath, [9, 19], exportedFunctions);
  assert.equal(resolvedBreakpoints[0].verified, true, 'Expected increment breakpoint to resolve');
  assert.equal(resolvedBreakpoints[0].functionName, 'increment');
  assert.equal(resolvedBreakpoints[1].verified, true, 'Expected get breakpoint to resolve');
  assert.equal(resolvedBreakpoints[1].functionName, 'get');

  await debuggerProcess.setBreakpoint('increment');
  const paused = await debuggerProcess.execute();
  assert.equal(paused.paused, true, 'Expected breakpoint to pause before execution');

  const resumed = await debuggerProcess.continueExecution();
  assert.match(resumed.output || '', /I64\(1\)/, 'Expected continue() to finish increment()');
  await debuggerProcess.clearBreakpoint('increment');

  const result = await debuggerProcess.execute();
  assert.match(result.output, /I64\(2\)/, 'Expected second increment() to return I64(2)');

  const inspection = await debuggerProcess.inspect();
  assert.ok(Array.isArray(inspection.callStack), 'Expected call stack array from inspection');

  const storage = await debuggerProcess.getStorage();
  assert.ok(typeof storage === 'object' && storage !== null, 'Expected storage snapshot object');

  await debuggerProcess.stop();
  console.log('VS Code extension smoke tests passed');
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
