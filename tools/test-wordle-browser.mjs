// Real Svelte/Chromium regressions. Mount the unmodified component with test
// props; replace native IPC only in this test's Vite server.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(new URL('../apps/desktop/package.json', import.meta.url));
const { chromium } = require('playwright');
const { createServer, normalizePath } = await import(pathToFileURL(require.resolve('vite')).href);
const root = fileURLToPath(new URL('../apps/desktop/', import.meta.url));
const mockBridge = `
  let command, id=0;
  window.wordleTest={hold:false,failure:null,commands:[],completed:0,waiting:false,release:null};
  async function pause(){if(window.wordleTest.hold)await new Promise(resolve=>{window.wordleTest.waiting=true;window.wordleTest.release=()=>{window.wordleTest.waiting=false;resolve();};});}
  export const message=String;
  export const bridge={
    wordlePreferences:async()=>{},wordleCancel:async()=>{},
    wordleStart:async c=>{
      command=c;window.wordleTest.commands.push(c);
      if(window.wordleTest.failure==='start'){
        await pause();throw new Error('Start failed');
      }
      return ++id;
    },
    wordleStatus:async()=>{
      const current=command,currentId=id,{failure}=window.wordleTest;
      await pause();
      if(failure==='status')throw new Error('Status failed');
      window.wordleTest.completed++;
      return {id:currentId,revision:current.revision,active:false,finished:false,
        history:current.action==='guess'?[{word:current.word,feedback:Array(current.request.length).fill('gray')}]:[],
        error:failure==='snapshot'?'Practice failed':null,
        analysis:failure==='snapshot'?null:{candidate_count:10,candidates:[],notices:[],
          profile:{name:'Browser test',fingerprint:'test',frequency_available:false},
          strategic:null,answer:null,attempts_remaining:current.attempts,recommendation_ms:0}
      };
    }
  };
`;

test('rendered Wordle keyboard, active cells and Practice focus recovery', { timeout: 90000 }, async t => {
  const server = await createServer({
    root,
    server: { host: '127.0.0.1', port: 0, strictPort: false },
    plugins: [{
      name: 'wordle-browser-test', enforce: 'pre',
      transform(code, id) {
        if (normalizePath(id) === `${normalizePath(root)}src/bridge.ts`) return { code: mockBridge, map: null };
        if (normalizePath(id) === `${normalizePath(root)}src/main.ts`) return { code: `
          import {mount} from 'svelte';
          import Wordle from './Wordle.svelte';
          import './style.css';
          mount(Wordle,{target:document.getElementById('app'),props:{
            settings:{wordle:{length:8,attempts:12,mode:'play'}},visible:true,frequencyAvailable:false
          }});
        `, map: null };
      },
    }],
  });
  t.after(() => server.close());
  await server.listen();
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  const url = server.resolvedUrls.local[0];
  await page.goto(url);
  assert.equal(await page.title(), 'TajsAnagrams');
  const grid = page.getByRole('grid');
  await grid.waitFor();
  await page.waitForFunction(() => window.wordleTest.completed > 0);
  assert.equal(await page.locator('vite-error-overlay').count(), 0);

  async function selected(row, column) {
    // Assert uniqueness over the actual rendered DOM, then resolve the active
    // descendant and compare it with the independently marked selected cell.
    const cells = await grid.getByRole('gridcell').evaluateAll(nodes => nodes.map(n => n.id));
    assert.equal(cells.length, 96);
    assert.equal(new Set(cells).size, cells.length);
    const state = await grid.evaluate(el => {
      const id = el.getAttribute('aria-activedescendant');
      const target = document.getElementById(id);
      const chosen = el.querySelectorAll('[aria-selected="true"]');
      return { found: !!target && el.contains(target), selected: chosen.length === 1 && target === chosen[0], label: target?.getAttribute('aria-label') };
    });
    assert.equal(state.found, true);assert.equal(state.selected, true);
    assert.match(state.label, new RegExp(`^Row ${row + 1}, position ${column + 1}:`));
  }
  await selected(0, 0);
  assert.equal(await grid.getByRole('gridcell').evaluateAll(nodes => nodes.every(n => n.tabIndex === -1)), true);
  // Enter through the preceding enabled control, then Tab out and back in.
  const newPuzzle = page.getByRole('button', { name: '+ New puzzle', exact: true });
  await newPuzzle.focus();await page.keyboard.press('Tab');
  assert.equal(await grid.evaluate(el => el === document.activeElement), true);
  await page.keyboard.press('Tab');
  assert.equal(await grid.evaluate(el => el.contains(document.activeElement)), false);
  await page.keyboard.press('Shift+Tab');
  assert.equal(await grid.evaluate(el => el === document.activeElement), true);
  await page.keyboard.type('ABCDEFGH');await selected(0, 0);
  await page.keyboard.press('End');await selected(0, 7);
  await page.keyboard.press('Home');await page.keyboard.press('ArrowRight');await selected(0, 1);
  await page.keyboard.press('Space');
  assert.match(await grid.getByRole('gridcell').nth(1).getAttribute('aria-label'), /B, gray/);
  await page.keyboard.press('ArrowDown');await selected(1, 1);
  await page.keyboard.press('ArrowUp');await selected(0, 1);
  await page.keyboard.press('Shift+Space');
  assert.match(await grid.getByRole('gridcell').nth(1).getAttribute('aria-label'), /B, unset/);

  await page.locator('.wordle-stale').waitFor({state:'hidden'});
  await page.locator('.wordle-assistant[aria-busy="false"]').waitFor();
  for (const failure of ['snapshot', 'start', 'status']) {
    for (const moved of [false, true]) {
      await page.getByRole('button', { name: 'Practice', exact: true }).click();
      await page.evaluate(failure => Object.assign(window.wordleTest, { hold: true, failure }), failure);
      const start = page.getByRole('button', { name: 'Start game', exact: true });
      await start.focus();await page.keyboard.press('Enter');
      await page.waitForFunction(() => window.wordleTest.waiting && document.querySelector('.practice-start button').disabled);
      const other = page.getByRole('button', { name: 'Practice', exact: true });
      if (moved) await other.focus();
      else await page.evaluate(() => document.activeElement.blur()); // model lost focus on disable
      await page.evaluate(() => window.wordleTest.release());
      await page.getByRole('alert').waitFor();
      await page.waitForFunction(() => !document.querySelector('.practice-start button').disabled);
      await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
      assert.equal(await (moved ? other : start).evaluate(el => el === document.activeElement), true, `${failure}, moved=${moved}`);
    }
  }
  await page.evaluate(() => Object.assign(window.wordleTest, { hold: false, failure: null }));
  await page.getByRole('button', { name: 'Start game', exact: true }).focus();await page.keyboard.press('Enter');
  await grid.waitFor();
  await page.waitForFunction(() => document.activeElement?.getAttribute('role') === 'grid');
  await selected(0, 0);
  await page.keyboard.type('ABCDEFGH');await page.evaluate(() => window.wordleTest.hold=true);await page.keyboard.press('Enter');
  await page.waitForFunction(() => window.wordleTest.waiting);
  const hint = page.getByRole('button', { name: 'Get a hint', exact: true });
  // The hint is disabled during a request; use the enabled mode control.
  const practice = page.getByRole('button', { name: 'Practice', exact: true });
  await practice.focus();await page.evaluate(() => window.wordleTest.release());
  await page.waitForFunction(() => document.querySelector('.practice-help h4')?.textContent === 'Keep going');
  assert.match(await grid.getByRole('gridcell').nth(8).getAttribute('aria-label'), /empty/);
  await selected(1, 0);
  assert.equal(await practice.evaluate(el => el === document.activeElement), true);
  assert.equal(await hint.isEnabled(), true);
  assert.deepEqual(errors, []);
});
