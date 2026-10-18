const { existsSync } = require('node:fs');
const { homedir } = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

function rustEnvironment(environment = process.env) {
  const env = { ...environment };
  if (process.platform !== 'win32') return env;

  const probe = spawnSync('cargo', ['--version'], { env, windowsHide: true });
  // Preserve an existing Cargo installation, including its own diagnostic errors.
  if (probe.error?.code !== 'ENOENT') return env;

  const cargoHome = env.CARGO_HOME || path.join(homedir(), '.cargo');
  const bin = path.resolve(cargoHome, 'bin');
  if (!existsSync(path.join(bin, 'cargo.exe'))) {
    throw new Error(`Cargo non trovato nel PATH o in ${bin}. Installa Rust tramite https://rustup.rs/ oppure imposta CARGO_HOME sulla tua installazione.`);
  }

  const pathKey = Object.keys(env).find(key => key.toLowerCase() === 'path') || 'Path';
  env[pathKey] = `${bin}${path.delimiter}${env[pathKey] || ''}`;
  return env;
}

if (require.main === module) {
  try {
    const args = process.argv.slice(2);
    const env = ['dev', 'build'].includes(args[0]) ? rustEnvironment() : process.env;
    const result = spawnSync(process.execPath, [require.resolve('@tauri-apps/cli/tauri.js'), ...args], {
      env,
      stdio: 'inherit',
      windowsHide: true,
    });
    if (result.error) throw result.error;
    process.exitCode = result.status ?? 1;
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}

module.exports = { rustEnvironment };
