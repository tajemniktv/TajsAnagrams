// Real Svelte/Chromium regressions. Mount the unmodified component with test
// props; replace native IPC only in this test's Vite server.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import fs from 'node:fs';

const require = createRequire(new URL('../apps/desktop/package.json', import.meta.url));
const { chromium } = require('playwright');
const { createServer, normalizePath } = await import(pathToFileURL(require.resolve('vite')).href);
const root = fileURLToPath(new URL('../apps/desktop/', import.meta.url));
const mockBridge = `
  let command, id=0;
  window.wordleTest={hold:false,failure:null,commands:[],completed:0,waiting:false,release:null};
  async function pause(){if(window.wordleTest.hold)await new Promise(resolve=>{window.wordleTest.waiting=true;window.wordleTest.release=()=>{window.wordleTest.waiting=false;resolve();};});}
  export const message=e=>e instanceof Error?e.message:String(e);
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
  const browser = await chromium.launch({ headless: true, executablePath: process.env.TAJS_BROWSER_EXECUTABLE });
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
  await page.setViewportSize({width:760,height:600});
  assert.equal(await page.locator('.wordle-shell').evaluate(el=>el.scrollWidth<=el.clientWidth),true,'8-letter / 12-attempt layout fits minimum width');
  assert.deepEqual(errors, []);
});

test('rendered desktop audit flows: presets, settings, worker ownership, custom lists and resume', {timeout:90000}, async t=>{
  const fixtures=JSON.parse(fs.readFileSync(new URL('../contracts/fixtures.json',import.meta.url),'utf8'));
  const request=fixtures.find(f=>f.schema==='SolveRequest'&&f.valid).value;
  Object.assign(request.generation,{text:'listen',min_words:1,max_words:6,min_word_length:1,max_word_length:24,min_zipf:1.8});
  const settings={schema_version:1,developer_mode:false,theme:'dark',corpora:{dictionary:'dictionary',unigrams:'',bigrams:'bigrams',wordnet:'wordnet',phrase:''},model:'',wordle:{length:5,attempts:6,mode:'play',play_profile:'broad',instant_profile:'broad',practice_profile:'broad',answers_path:'',guesses_path:''},request,runtime:{cache_path:'',cache_entries:32,cache_bytes:33554432,custom_limits:false,limits:{}}};
  const mock=`
    let settings=JSON.parse(localStorage.getItem('audit-test-settings')||${JSON.stringify(JSON.stringify(settings))});
    const words=Array.from({length:350},(_,n)=>{let word='';for(let i=0;i<5;i++){word+=String.fromCharCode(97+n%26);n=Math.floor(n/26);}return word;});
    let command,id=10;
    window.auditTest={commands:[],copied:null,operation:null,hold:false,waiting:false,release:null,path:'alpha.txt'};
    const state=window.auditTest;
    const status={state:'completed',stage:'complete',exhaustion:'exhausted',cache:{hit:false},counts:{generated:2,deep_analyzed:2,shown:2,orders_evaluated:2}};
    export const message=e=>e instanceof Error?e.message:String(e);
    export const bridge={
      bootstrap:async()=>({settings,corpus_error:null,wordle_frequencies_available:false,data_directory:'test',limits:{}}),
      save:async next=>{settings=next;localStorage.setItem('audit-test-settings',JSON.stringify(settings));},
      status:async()=>({id:1,active:false,status,error:null}),
      result:async()=>({kind:'ranked',buckets:{1:[{best_order:['listen'],display_phrase:'listen',final:1},{best_order:['silent'],display_phrase:'silent',final:0}]},status}),
      operationStatus:async()=>state.operation,cancelOperation:async()=>state.operation=null,
      choose:async()=>state.path,copyText:async text=>state.copied=text,clearCache:async()=>{},
      maintenanceStatus:async()=>({active:false,error:null,report:null}),
      wordlePreferences:async prefs=>{settings.wordle=prefs;localStorage.setItem('audit-test-settings',JSON.stringify(settings));},
      wordleCancel:async()=>{if(state.release)state.release();},
      wordleStart:async c=>{command=c;state.commands.push(c);return ++id;},
      wordleStatus:async()=>{
        const c=command,currentId=id;
        if(state.hold)await new Promise(resolve=>{state.waiting=true;state.release=()=>{state.hold=false;state.waiting=false;resolve();};});
        const error=c.profile==='custom'&&!c.answers_path?'Choose an answer list':null;
        return {id:currentId,revision:c.revision,mode:c.mode,active:false,error,history:[],finished:false,analysis:error?null:{candidate_count:words.length,candidates:words,notices:[],profile:{name:c.profile==='custom'?c.answers_path:'Broad English',fingerprint:'vocab-'+c.answers_path,frequency_available:false},strategic:null,answer:null,attempts_remaining:c.attempts,recommendation_ms:0}};
      }
    };
  `;
  const server=await createServer({root,server:{host:'127.0.0.1',port:0},plugins:[{name:'audit-browser-test',enforce:'pre',transform(code,id){if(normalizePath(id)===`${normalizePath(root)}src/bridge.ts`)return {code:mock,map:null};}}]});
  t.after(()=>server.close());await server.listen();
  const browser=await chromium.launch({headless:true,executablePath:process.env.TAJS_BROWSER_EXECUTABLE});t.after(()=>browser.close());
  const page=await browser.newPage({viewport:{width:1280,height:900}}),errors=[];
  page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(['error','warning'].includes(m.type()))errors.push(m.text());});
  await page.goto(server.resolvedUrls.local[0]);assert.equal(await page.title(),'TajsAnagrams');
  await page.getByRole('heading',{name:'Find the words'}).waitFor();
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'Settings',exact:true});await dialog.waitFor();
  assert.equal(await dialog.getByRole('tab').count(),2);
  assert.equal(await dialog.getByLabel('Dictionary',{exact:true}).count(),0);
  await dialog.getByLabel('Developer tools',{exact:true}).check();assert.equal(await dialog.getByRole('tab').count(),5);
  await dialog.getByRole('tab',{name:'General',exact:true}).focus();await page.keyboard.press('End');
  assert.equal(await dialog.getByRole('tab',{name:'Diagnostics'}).getAttribute('aria-selected'),'true');
  await page.keyboard.press('Home');await dialog.getByLabel('Developer tools',{exact:true}).uncheck();
  await dialog.getByRole('button',{name:'Save settings'}).click();await dialog.waitFor({state:'hidden'});
  await page.getByText('Advanced options',{exact:true}).click();
  for(const preset of ['quick','balanced','exhaustive']){
    await page.getByLabel(/^Search preset/).selectOption(preset);
    for(const [label,expected] of [['Minimum words','1'],['Minimum length','1'],['Minimum frequency (Zipf)','1.8']])assert.equal(await page.getByLabel(label,{exact:true}).inputValue(),expected);
  }
  await page.getByRole('button',{name:'Copy listen',exact:true}).click();assert.equal(await page.evaluate(()=>window.auditTest.copied),'listen');
  await page.getByRole('button',{name:'Refine',exact:true}).first().click();await page.getByRole('button',{name:'Require listen',exact:true}).click();
  assert.equal(await page.getByLabel('Required words',{exact:false}).inputValue(),'listen');
  await page.getByRole('button',{name:'Exclude listen',exact:true}).click();assert.equal(await page.getByLabel('Exclude words',{exact:false}).inputValue(),'listen');
  assert.equal(await page.getByLabel('Required words',{exact:false}).inputValue(),'');
  await page.evaluate(()=>window.auditTest.operation={id:99,kind:'anagrams',cancelling:false});
  await page.getByText('Anagram search is running',{exact:true}).waitFor();
  await page.getByRole('button',{name:'Wordle',exact:true}).click();
  await page.getByText('Another tool is using the worker.',{exact:false}).waitFor();
  assert.equal(await page.evaluate(()=>window.auditTest.commands.length),0);
  await page.getByRole('button',{name:'Cancel operation',exact:true}).click();
  await page.waitForFunction(()=>window.auditTest.commands.length>0);
  const setup=page.locator('.wordle-setup');await setup.locator('summary').click();
  await page.getByLabel(/^Vocabulary/).selectOption('custom');
  await page.getByRole('alert').filter({hasText:'Choose an answer list'}).waitFor();
  const chooser=page.locator('.wordle-custom').getByRole('button',{name:'Choose…'}).first();
  await chooser.click();await page.getByText('alpha.txt · equally likely answers',{exact:false}).waitFor();
  await page.evaluate(()=>window.auditTest.path='beta.txt');await chooser.click();
  await page.getByText('beta.txt · equally likely answers',{exact:false}).waitFor();
  assert.equal(await page.getByText('alpha.txt · equally likely answers',{exact:false}).count(),0);
  await page.getByText('Remaining words',{exact:true}).click();
  const region=page.getByRole('region',{name:'Candidate words'}),grid=page.getByRole('grid');
  const activeCell=await grid.getAttribute('aria-activedescendant');await region.focus();await page.keyboard.press('ArrowDown');
  await page.waitForFunction(()=>document.querySelector('.candidate-scroll').scrollTop>0);
  assert.equal(await grid.getAttribute('aria-activedescendant'),activeCell);
  await region.evaluate(el=>el.scrollTop=9000);await page.waitForFunction(()=>Number(document.querySelector('.candidate-scroll tbody tr[aria-rowindex]')?.getAttribute('aria-rowindex'))>100);
  await grid.focus();await page.keyboard.type('APPLE');await page.keyboard.press('Space');
  await page.getByRole('button',{name:'Undo edit',exact:true}).click();
  assert.match(await grid.getByRole('gridcell').first().getAttribute('aria-label'),/A, unset/);
  await grid.focus();await page.keyboard.press('Space');
  await page.waitForFunction(()=>JSON.parse(localStorage.getItem('tajs-wordle-session-v1')).play.marks[0]==='gray');
  await page.reload();await page.getByRole('button',{name:'Wordle',exact:true}).click();
  await page.getByRole('grid').waitFor();assert.match(await page.getByRole('gridcell').first().getAttribute('aria-label'),/A, gray/);
  await page.waitForFunction(()=>window.auditTest.commands.length>0);
  assert.equal(await page.evaluate(()=>window.auditTest.commands.at(-1).expected_fingerprint),'vocab-beta.txt');
  await page.evaluate(()=>window.auditTest.hold=true);
  await page.getByRole('grid').focus();await page.keyboard.press('Space');
  await page.waitForFunction(()=>window.auditTest.waiting);
  await page.getByRole('button',{name:'Instant Solver',exact:true}).click();
  await page.waitForFunction(()=>window.auditTest.commands.at(-1).mode==='instant'&&!window.auditTest.waiting);
  await page.getByRole('heading',{name:'Matching words',exact:true}).waitFor();
  assert.equal(await page.getByText('beta.txt · equally likely answers',{exact:false}).count(),0,'late Play response stays in its own mode');
  await page.getByLabel('Letter to count',{exact:true}).fill('s');await page.getByRole('button',{name:'Add letter',exact:true}).click();
  await page.getByLabel('Count 1 minimum',{exact:true}).fill('0');await page.getByLabel('Count 1 maximum',{exact:true}).fill('0');await page.getByLabel('Count 1 maximum',{exact:true}).blur();
  await page.waitForFunction(()=>window.auditTest.commands.at(-1).request.constraints.counts.some(c=>c.letter==='s'&&c.min===0&&c.max===0));
  assert.equal(await page.locator('vite-error-overlay').count(),0);assert.deepEqual(errors,[]);
  if(process.env.TAJS_QA_DIR){fs.mkdirSync(process.env.TAJS_QA_DIR,{recursive:true});await page.screenshot({path:`${process.env.TAJS_QA_DIR}/instant-desktop.png`});await page.setViewportSize({width:760,height:600});await page.evaluate(()=>document.documentElement.dataset.theme='light');await page.screenshot({path:`${process.env.TAJS_QA_DIR}/instant-minimum-light.png`});}
  await page.addInitScript(()=>localStorage.setItem('tajs-wordle-session-v1','{"schema":999}'));
  await page.reload();await page.getByRole('button',{name:'Wordle',exact:true}).click();
  await page.getByText('Saved Wordle puzzle is incompatible. Start a new puzzle.',{exact:true}).waitFor();
  assert.equal(await page.locator('vite-error-overlay').count(),0);assert.deepEqual(errors,[]);
});
