import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import { DebuggerProcess } from '../cli/debuggerProcess';

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

  const result = await debuggerProcess.execute();
  assert.match(result.output, /I64\(1\)/, 'Expected increment() to return I64(1)');

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
