// Content-checked local Vite cache. Leaving unchanged dist files untouched also
// avoids invalidating the Rust host that embeds them. Type checks still run.
import { createHash } from 'node:crypto';
import * as fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(import.meta.url);
const defaultProject = path.resolve(path.dirname(script), '..');

function tree(directory, exclude = new Set()) {
  if (!fs.existsSync(directory)) return [];
  const files = [];
  function visit(relative) {
    for (const name of fs.readdirSync(path.join(directory, relative)).sort()) {
      if (!relative && exclude.has(name)) continue;
      const entry = path.join(relative, name);
      const stat = fs.lstatSync(path.join(directory, entry));
      if (stat.isSymbolicLink()) throw new Error(`Unsupported symlink in build inputs/outputs: ${entry}`);
      if (stat.isDirectory()) visit(entry);
      else if (stat.isFile()) {
        files.push([entry.replaceAll('\\', '/'),
          createHash('sha256').update(fs.readFileSync(path.join(directory, entry))).digest('hex')]);
      }
    }
  }
  visit('');
  return files;
}

function digest(value) {
  return createHash('sha256').update(JSON.stringify(value)).digest('hex');
}

export function buildFrontend({ project = defaultProject, env = process.env, args = [], build } = {}) {
  const desktop = path.join(project, 'apps/desktop');
  const dist = path.join(desktop, 'dist');
  const stamp = path.join(project, '.codex/temp/frontend-build.json');
  const runBuild = () => {
    if (build) build();
    else {
      const result = spawnSync(process.execPath,
        [path.join(desktop, 'node_modules/vite/bin/vite.js'), 'build', ...args],
        { cwd: desktop, env, stdio: 'inherit' });
      if (result.error) throw result.error;
      if (result.status !== 0) throw new Error(`Vite build failed (${result.status ?? result.signal})`);
    }
  };
  // Custom flags can change the output directory or enable watch/SSR builds.
  // Let Vite own those cases rather than caching the default dist by mistake.
  if (args.length) {
    console.log('Frontend cache bypass: custom Vite arguments.');
    runBuild();
    return { cached: false };
  }
  const input = () => digest({
    schema: 1,
    script: fs.readFileSync(script, 'utf8'),
    runtime: [process.version, process.platform, process.arch],
    args,
    // Vite exposes VITE_*; Tauri and Node also pass build/runtime settings.
    env: Object.entries(env).filter(([key]) => /^(VITE_|TAURI_|NODE_)/.test(key)).sort(),
    desktop: tree(desktop, new Set(['node_modules', 'dist', 'src-tauri'])),
    contracts: tree(path.join(project, 'contracts/generated')),
  });
  const before = input();
  let previous;
  try { previous = JSON.parse(fs.readFileSync(stamp, 'utf8')); }
  catch (error) {
    if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
  }
  if (previous?.input === before && fs.existsSync(path.join(dist, 'index.html'))
      && previous.output === digest(tree(dist))) {
    console.log('Frontend cache hit: inputs and outputs match; dist left untouched.');
    return { cached: true };
  }
  console.log('Frontend cache miss: rebuilding with Vite.');
  runBuild();
  if (!fs.existsSync(path.join(dist, 'index.html'))) throw new Error('Vite output index.html is missing');
  if (input() !== before) throw new Error('Frontend inputs changed during the build; retry');
  fs.mkdirSync(path.dirname(stamp), { recursive: true });
  const next = `${stamp}.${process.pid}.next`;
  try {
    fs.writeFileSync(next, JSON.stringify({ input: before, output: digest(tree(dist)) }));
    fs.renameSync(next, stamp);
  } finally {
    if (fs.existsSync(next)) fs.unlinkSync(next);
  }
  return { cached: false };
}

if (process.argv[1] && path.resolve(process.argv[1]) === script) {
  buildFrontend({ args: process.argv.slice(2) });
}
