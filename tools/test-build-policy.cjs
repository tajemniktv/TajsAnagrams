const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..');
function fixture(t) {
  const scratch = path.join(root, '.codex/temp');
  fs.mkdirSync(scratch, { recursive: true });
  const project = fs.mkdtempSync(path.join(scratch, 'frontend-cache-test-'));
  t.after(() => fs.rmSync(project, { recursive: true, force: true }));
  const write = (name, text) => {
    const file = path.join(project, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, text);
  };
  write('apps/desktop/src/App.svelte', 'original source');
  write('apps/desktop/package.json', '{}');
  let builds = 0;
  const options = { project, env: {}, build: () => {
    builds++;
    write('apps/desktop/dist/index.html', `built ${builds}`);
    write('apps/desktop/dist/assets/app.js', `script ${builds}`);
  } };
  return { project, write, options, count: () => builds };
}

test('unchanged inputs reuse verified outputs without touching mtimes', async t => {
  const { buildFrontend } = await import('./build-frontend.mjs');
  const f = fixture(t);
  assert.equal(buildFrontend(f.options).cached, false);
  assert.equal(fs.existsSync(path.join(f.project, 'build/frontend-build.json')), true);
  assert.equal(fs.existsSync(path.join(f.project, '.codex/temp/frontend-build.json')), false);
  const file = path.join(f.project, 'apps/desktop/dist/index.html');
  fs.utimesSync(file, 1000, 1000);
  const before = fs.statSync(file).mtimeMs;
  assert.equal(buildFrontend(f.options).cached, true);
  assert.equal(fs.statSync(file).mtimeMs, before);
  assert.equal(f.count(), 1);
});

for (const name of ['src/App.svelte', 'public/logo.png', 'vite.config.ts',
  'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'package.json', '.env.production']) {
  test(`${name} content changes invalidate even with an unchanged mtime`, async t => {
    const { buildFrontend } = await import('./build-frontend.mjs');
    const f = fixture(t);
    const relative = `apps/desktop/${name}`;
    f.write(relative, 'before');
    buildFrontend(f.options);
    const file = path.join(f.project, relative);
    const mtime = fs.statSync(file).mtime;
    f.write(relative, 'after');
    fs.utimesSync(file, mtime, mtime);
    assert.equal(buildFrontend(f.options).cached, false);
    assert.equal(f.count(), 2);
  });
}

test('added/deleted sources, shared contracts and build environment invalidate', async t => {
  const { buildFrontend } = await import('./build-frontend.mjs');
  const f = fixture(t);
  buildFrontend(f.options);
  f.write('apps/desktop/src/extra.ts', 'extra');
  assert.equal(buildFrontend(f.options).cached, false);
  fs.unlinkSync(path.join(f.project, 'apps/desktop/src/extra.ts'));
  assert.equal(buildFrontend(f.options).cached, false);
  f.write('contracts/generated/Request.ts', 'shared');
  assert.equal(buildFrontend(f.options).cached, false);
  f.options.env.VITE_FEATURE = 'enabled';
  assert.equal(buildFrontend(f.options).cached, false);
  // Rust edits and generated Rust files do not affect the frontend bundle.
  f.write('apps/desktop/src-tauri/src/main.rs', 'rust-only');
  assert.equal(buildFrontend(f.options).cached, true);
});

test('custom Vite arguments bypass the default-output cache', async t => {
  const { buildFrontend } = await import('./build-frontend.mjs');
  const f = fixture(t);
  const options = { ...f.options, args: ['--outDir', 'other'] };
  assert.equal(buildFrontend(options).cached, false);
  assert.equal(buildFrontend(options).cached, false);
  assert.equal(f.count(), 2);
  assert.equal(fs.existsSync(path.join(f.project, 'build/frontend-build.json')), false);
});

test('missing, altered and extra outputs or corrupt metadata cause a rebuild', async t => {
  const { buildFrontend } = await import('./build-frontend.mjs');
  const f = fixture(t);
  buildFrontend(f.options);
  fs.unlinkSync(path.join(f.project, 'apps/desktop/dist/assets/app.js'));
  assert.equal(buildFrontend(f.options).cached, false);
  f.write('apps/desktop/dist/index.html', 'corrupt');
  assert.equal(buildFrontend(f.options).cached, false);
  f.write('apps/desktop/dist/extra.txt', 'unexpected');
  assert.equal(buildFrontend(f.options).cached, false);
  f.write('build/frontend-build.json', 'broken JSON');
  assert.equal(buildFrontend(f.options).cached, false);
});

test('failed builds and edits during a build never publish a valid cache entry', async t => {
  const { buildFrontend } = await import('./build-frontend.mjs');
  const f = fixture(t);
  assert.throws(() => buildFrontend({ ...f.options, build: () => { throw Error('failed'); } }), /failed/);
  assert.equal(fs.existsSync(path.join(f.project, 'build/frontend-build.json')), false);
  const build = f.options.build;
  assert.throws(() => buildFrontend({ ...f.options, build: () => {
    build();
    f.write('apps/desktop/src/App.svelte', 'edited during build');
  } }), /changed during/);
  assert.equal(fs.existsSync(path.join(f.project, 'build/frontend-build.json')), false);
  assert.equal(buildFrontend(f.options).cached, false);
});
