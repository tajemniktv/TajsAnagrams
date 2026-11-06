// Exercise actual component event/state code, including asynchronous IPC races.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {test}=require('node:test');
const ts=require('../apps/desktop/node_modules/typescript');
function deferred(){let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};}
function harness(mode='play'){
  const status=deferred();let starts=0;
  const preferences={length:5,attempts:6,mode,play_profile:'broad',instant_profile:'broad',practice_profile:'broad',answers_path:'',guesses_path:''};
  const source=fs.readFileSync(path.join(__dirname,'../apps/desktop/src/Wordle.svelte'),'utf8').split('<script lang="ts">')[1].split('</script>')[0].replace(/^\s*import .*;$/gm,'');
  const state=Object.assign(x=>x,{snapshot:structuredClone}),derived=Object.assign(x=>x,{by:fn=>fn()});
  const context=vm.createContext({$state:state,$derived:derived,$bindable:x=>x,$props:()=>({settings:{wordle:preferences},visible:true,frequencyAvailable:false}),$effect:()=>{},untrack:fn=>fn(),onMount:()=>{},structuredClone,document:{querySelector:()=>null},requestAnimationFrame:fn=>fn(),setTimeout:()=>1,clearTimeout:()=>{},bridge:{wordleStart:async()=>{starts++;return 7;},wordleStatus:()=>status.promise,wordlePreferences:async()=>{}},message:String});
  const expose='globalThis.api={accept,run,submit,typeKey,keydown,changeDraft,useWord,clueKey,get historyOpen(){return historyOpen},get boards(){return boards},get active(){return active},get starts(){return 0},dispose(){disposed=true}};';
  vm.runInContext(ts.transpileModule(source+'\n'+expose,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText,context);
  return {api:context.api,status,starts:()=>starts};
}
test('incomplete feedback never submits or becomes gray',async()=>{const h=harness();h.api.changeDraft('APPLE');await h.api.submit();assert.equal(h.starts(),0);assert.match(h.api.boards.play.error,/every tile/);assert.ok(h.api.boards.play.marks.every(m=>m===null));});
test('changing a guess clears previous feedback',()=>{const h=harness();h.api.boards.play.marks.fill('green');h.api.changeDraft('ALLEY');assert.ok(h.api.boards.play.marks.every(m=>m===null));});
test('late analysis cannot replace an edited revision or another mode',()=>{const h=harness();h.api.boards.play.revision=3;h.api.accept({revision:2,analysis:{candidate_count:99},error:'stale'},'play');assert.equal(h.api.boards.play.analysis,null);assert.equal(h.api.boards.play.error,'');h.api.accept({revision:3,analysis:{candidate_count:7}},'play');assert.equal(h.api.boards.play.analysis.candidate_count,7);assert.equal(h.api.boards.practice.analysis,null);});
test('disposing while IPC is pending prevents late publication',async()=>{const h=harness();const pending=h.api.run('analyze');await new Promise(setImmediate);h.api.dispose();h.status.resolve({id:7,revision:1,active:false,analysis:{candidate_count:99}});await pending;assert.equal(h.api.boards.play.analysis,null);});

test('using an Instant Solver suggestion opens its feedback editor without submitting',()=>{
  const h=harness('instant');h.api.useWord('apple');
  assert.equal(h.api.boards.instant.draft,'APPLE');assert.equal(h.api.historyOpen,true);
  assert.ok(h.api.boards.instant.marks.every(m=>m===null));assert.equal(h.starts(),0);
});

test('Instant Solver keeps the analyzed clue identity when candidate pages arrive',async()=>{
  const h=harness('instant'),board=h.api.boards.instant;
  const pending=h.api.run('analyze');await new Promise(setImmediate);
  h.status.resolve({id:7,revision:1,active:false,analysis:{candidate_count:99}});await pending;
  const original=board.analyzedClues;assert.equal(original,h.api.clueKey(board));
  board.fixed='a....';assert.notEqual(original,h.api.clueKey(board));
  h.api.accept({revision:1,analysis:{candidate_count:99,offset:100}},'instant');
  assert.equal(board.analyzedClues,original);assert.notEqual(board.analyzedClues,h.api.clueKey(board));
});

test('physical typing opens Instant Solver history while Enter on disclosures keeps its native action',()=>{
  const h=harness('instant');let prevented=false;
  h.api.keydown({key:'Enter',target:{closest:selector=>selector==='button,summary'},preventDefault(){prevented=true;}});
  assert.equal(prevented,false);assert.equal(h.api.historyOpen,false);assert.equal(h.starts(),0);
  h.api.keydown({key:'a',target:{closest:()=>false},preventDefault(){prevented=true;}});
  assert.equal(prevented,true);assert.equal(h.api.historyOpen,true);assert.equal(h.api.boards.instant.draft,'A');
});
