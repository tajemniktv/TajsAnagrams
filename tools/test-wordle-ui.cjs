// Exercise the component's actual input/session code with controlled IPC timing.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {test}=require('node:test');
const ts=require('../apps/desktop/node_modules/typescript');
function deferred(){let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};}
const flush=()=>new Promise(setImmediate);
function harness(mode='play'){
  const commands=[],statuses=[],cancelled=[],timers=new Map();let timerId=0;
  const preferences={length:5,attempts:6,mode,play_profile:'broad',instant_profile:'broad',practice_profile:'broad',answers_path:'',guesses_path:''};
  const source=fs.readFileSync(path.join(__dirname,'../apps/desktop/src/Wordle.svelte'),'utf8').split('<script lang="ts">')[1].split('</script>')[0].replace(/^\s*import .*;$/gm,'');
  const state=Object.assign(x=>x,{snapshot:structuredClone}),derived=Object.assign(x=>x,{by:fn=>fn()});
  const context=vm.createContext({$state:state,$derived:derived,$bindable:x=>x,$props:()=>({settings:{wordle:preferences},visible:true,frequencyAvailable:false}),$effect:()=>{},untrack:fn=>fn(),onMount:()=>{},structuredClone,document:{querySelector:()=>null},requestAnimationFrame:fn=>fn(),setTimeout:(fn,ms)=>{timers.set(++timerId,{fn,ms});return timerId;},clearTimeout:id=>timers.delete(id),bridge:{wordleStart:async command=>{commands.push(structuredClone(command));statuses.push(deferred());return commands.length;},wordleStatus:()=>statuses.at(-1).promise,wordleCancel:async id=>{cancelled.push(id);},wordlePreferences:async()=>{}},message:String});
  const expose=`globalThis.api={accept,run,submit,typeKey,keydown,changeDraft,useWord,clueKey,effectiveHistory,effectiveRows,partialHistory,tileClick,setFeedback,editRow,selectRow,removeRow,nextHint,drain,get board(){return b},get boards(){return boards},get active(){return active},view(query,order){search=query;sort=order;return candidateWords()},dispose(){disposed=true}};`;
  vm.runInContext(ts.transpileModule(source+'\n'+expose,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText,context);
  return {api:context.api,commands,statuses,cancelled,
    update(){for(const [id,t] of [...timers])if(t.ms===90){timers.delete(id);t.fn();}},
    complete(data={},i=commands.length-1){statuses[i].resolve({id:i+1,revision:commands[i].revision,active:false,history:[],finished:false,error:null,...data});}};
}
function enterWord(h,word='APPLE'){for(const letter of word)h.api.typeKey(letter);}
function markRow(h){for(let i=0;i<5;i++)h.api.tileClick(0,i);}

test('board typing replaces the selected letter and clears obsolete feedback',()=>{
  const h=harness();enterWord(h);assert.equal(h.api.board.draft,'APPLE');
  h.api.tileClick(0,1);assert.equal(h.api.board.marks[1],'gray');h.api.typeKey('X');
  assert.equal(h.api.board.draft,'AXPLE');assert.ok(h.api.board.marks.every(m=>m===null));
});
test('Delete clears a selected cell without shifting; Backspace always removes the last letter',()=>{
  const h=harness();enterWord(h);h.api.board.cursor=2;h.api.typeKey('DELETE');
  assert.equal(h.api.board.draft,'AP LE');h.api.typeKey('P');assert.equal(h.api.board.draft,'APPLE');
  h.api.board.cursor=0;h.api.typeKey('BACKSPACE');assert.equal(h.api.board.draft,'APPL');assert.equal(h.api.board.cursor,4);
});
test('Enter finishes a word but never treats unset feedback as gray',async()=>{
  const h=harness();enterWord(h);await h.api.submit();assert.equal(h.api.board.ready,true);
  assert.equal(h.api.effectiveHistory(h.api.board).length,0);assert.ok(h.api.board.marks.every(m=>m===null));assert.equal(h.commands.length,0);
});
test('direct typing and live feedback support every configured word length',()=>{
  for(let length=4;length<=8;length++){
    const h=harness();h.api.board.length=length;h.api.board.marks=Array(length).fill(null);
    const word='ABCDEFGH'.slice(0,length);enterWord(h,word);assert.equal(h.api.board.draft,word);
    for(let i=0;i<length;i++)h.api.tileClick(0,i);
    assert.equal(h.api.effectiveHistory(h.api.board)[0].feedback.length,length);
  }
});
test('tile feedback cycles forward and backward, including unset',()=>{
  const h=harness();enterWord(h);
  for(const expected of ['gray','yellow','green',null]){h.api.tileClick(0,0);assert.equal(h.api.board.marks[0],expected);}
  for(const expected of ['green','yellow','gray',null]){h.api.tileClick(0,0,-1);assert.equal(h.api.board.marks[0],expected);}
});
test('complete feedback is analyzed live, then Enter commits it exactly once',async()=>{
  const h=harness();enterWord(h);markRow(h);
  assert.equal(h.api.board.history.length,0);assert.equal(h.api.effectiveHistory(h.api.board).length,1);
  h.update();await flush();assert.equal(h.commands.length,1);assert.equal(h.commands[0].request.history[0].word,'apple');assert.equal(h.commands[0].request.attempts_remaining,5);
  await h.api.submit();assert.equal(h.api.board.entries[0].word,'APPLE');assert.equal(h.api.board.activeRow,1);assert.equal(h.api.effectiveHistory(h.api.board).length,1);assert.equal(h.api.board.draft,'');
});
test('typing an unmarked draft does not cancel an analysis of unchanged clues',async()=>{
  const h=harness();const pending=h.api.run('analyze');await flush();enterWord(h);
  assert.equal(h.cancelled.length,0);h.complete({analysis:{candidate_count:9}});await pending;
  assert.equal(h.api.board.analysis.candidate_count,9);assert.equal(h.api.board.analyzedClues,h.api.clueKey(h.api.board));
});
test('edits during analysis cancel and coalesce; stale responses never overwrite current feedback',async()=>{
  const h=harness();enterWord(h);markRow(h);h.update();await flush();
  h.api.tileClick(0,0,-1);h.api.tileClick(0,1,-1);assert.ok(h.cancelled.includes(1));
  h.complete({analysis:{candidate_count:999}},0);await flush();
  assert.equal(h.api.board.analysis,null);assert.equal(h.commands.length,2);assert.equal(h.commands[1].request.history.length,0);
  h.complete({analysis:{candidate_count:7}},1);await flush();assert.equal(h.api.board.analysis.candidate_count,7);
});
test('switching rows saves words and feedback without Enter or replaying feedback',()=>{
  const h=harness();enterWord(h);markRow(h);h.api.tileClick(1,0);
  assert.equal(h.api.board.entries[0].word,'APPLE');assert.equal(h.api.board.activeRow,1);
  enterWord(h,'STONE');h.api.tileClick(1,2);h.api.tileClick(0,2);
  assert.equal(h.api.board.draft,'APPLE');assert.ok(h.api.board.marks.every(m=>m==='gray'));
  h.api.tileClick(1,4);assert.equal(h.api.board.draft,'STONE');assert.equal(h.api.board.marks[2],'gray');assert.equal(h.api.board.marks[4],null);
});
test('right clicking a future empty row never clones the word or creates null history entries',async()=>{
  const h=harness();enterWord(h,'WARES');markRow(h);h.api.tileClick(4,1,-1);
  assert.equal(h.api.board.activeRow,4);assert.equal(h.api.board.draft,'');assert.ok(h.api.board.marks.every(m=>m===null));
  h.api.tileClick(4,2,-1);h.update();await flush();
  const request=JSON.parse(JSON.stringify(h.commands.at(-1).request));
  assert.equal(request.history.length,1);assert.equal(request.history[0].word,'wares');assert.equal(request.partial_history.length,0);
  assert.ok(request.history.every(row=>row&&typeof row.word==='string'));
});
test('invalid row and column events leave selection and drafts untouched',()=>{
  const h=harness();enterWord(h);const before=JSON.stringify(h.api.board);
  for(const index of [-1,6,99,NaN]){h.api.editRow(index);h.api.tileClick(index,0,-1);}
  for(const column of [-1,5,NaN])h.api.tileClick(0,column,-1);
  assert.equal(JSON.stringify(h.api.board),before);
});
test('one known tile filters live and survives row switching; unset is sent as unknown',async()=>{
  const h=harness();enterWord(h);h.api.tileClick(0,2);h.api.selectRow(3);h.api.selectRow(0);
  assert.equal(h.api.board.draft,'APPLE');assert.equal(h.api.board.marks[2],'gray');assert.equal(h.api.board.marks[0],null);
  h.update();await flush();const request=h.commands[0].request;
  assert.equal(request.history.length,0);assert.equal(request.partial_history.length,1);
  assert.equal(request.partial_history[0].feedback[0],null);assert.equal(request.partial_history[0].feedback[2],'gray');assert.equal(request.attempts_remaining,5);
});
test('unfinished word fragments and their exact positions survive row switching',()=>{
  const h=harness();enterWord(h,'AP');h.api.selectRow(2);enterWord(h,'ST');h.api.selectRow(0);
  assert.equal(h.api.board.draft,'AP');assert.equal(h.api.effectiveRows(h.api.board).length,0);
  h.api.selectRow(2);assert.equal(h.api.board.draft,'ST');
});
test('using a recommendation fills the board and leaves submission to the user',()=>{
  const h=harness();h.api.useWord('apple');assert.equal(h.api.board.draft,'APPLE');assert.equal(h.api.board.cursor,0);
  assert.ok(h.api.board.marks.every(m=>m===null));assert.equal(h.commands.length,0);
});
test('Use word preserves completed live feedback and fills the next row',()=>{
  const h=harness();enterWord(h);markRow(h);h.api.useWord('crane');
  assert.equal(h.api.board.entries[0].word,'APPLE');assert.equal(h.api.board.activeRow,1);
  assert.equal(h.api.board.draft,'CRANE');assert.ok(h.api.board.marks.every(m=>m===null));
  assert.equal(h.api.effectiveHistory(h.api.board).length,1);assert.equal(h.commands.length,0);
});
test('Practice hints start with guidance and request a word only on the second hint',async()=>{
  const h=harness('practice');h.api.board.started=true;
  h.api.nextHint();assert.equal(h.api.board.hints,1);assert.equal(h.commands.length,0);
  h.api.nextHint();await flush();assert.equal(h.api.board.hints,2);assert.equal(h.commands[0].action,'analyze');
});
test('Practice Enter submits to Rust; new feedback and history come only from Rust',async()=>{
  const h=harness('practice');h.api.board.started=true;h.api.board.hints=2;enterWord(h);const pending=h.api.submit();await flush();
  assert.equal(h.commands[0].action,'guess');assert.equal(h.api.board.history.length,0);
  h.complete({analysis:{candidate_count:2},history:[{word:'apple',feedback:Array(5).fill('yellow')} ]});await pending;
  assert.equal(h.api.board.history.length,1);assert.equal(h.api.board.draft,'');assert.equal(h.api.board.hints,0);assert.equal(h.api.board.revealed,null);
});
test('full candidate search and sorts cover words beyond the old 100-result page',()=>{
  const h=harness('instant');const words=Array.from({length:150},(_,i)=>'aaa'+String.fromCharCode(97+Math.floor(i/26),97+i%26));
  words.push('apple','alley','crane');h.api.board.analysis={candidates:words};
  assert.equal(h.api.view('apple','frequency')[0],'apple');assert.equal(h.api.view('','frequency').length,153);
  assert.equal(h.api.view('','az')[0],'aaaaa');assert.equal(h.api.view('','za')[0],'crane');assert.equal(h.api.view('','distinct')[0],'crane');assert.equal(words[0],'aaaaa');
});
test('Instant Solver does not capture letters intended for its constraint form',()=>{
  const h=harness('instant');let prevented=false;h.api.keydown({key:'a',target:{closest:()=>false},preventDefault(){prevented=true;}});
  assert.equal(prevented,false);assert.equal(h.api.board.draft,'');
});
test('keyboard arrows and Delete edit the board; Enter on disclosures keeps native behavior',()=>{
  const h=harness();enterWord(h);let prevented=false;
  h.api.keydown({key:'ArrowLeft',target:{closest:()=>false},preventDefault(){prevented=true;}});assert.equal(prevented,true);assert.equal(h.api.board.cursor,4);
  h.api.keydown({key:'Delete',target:{closest:()=>false},preventDefault(){}});assert.equal(h.api.board.draft,'APPL');
  prevented=false;h.api.keydown({key:'Enter',target:{closest:s=>s==='input,textarea,select,summary'},preventDefault(){prevented=true;}});assert.equal(prevented,false);
});
test('late analysis cannot replace an edited revision or another mode',()=>{
  const h=harness();h.api.board.revision=3;h.api.accept({revision:2,analysis:{candidate_count:99},error:'stale'},'play');
  assert.equal(h.api.board.analysis,null);assert.equal(h.api.board.error,'');h.api.accept({revision:3,analysis:{candidate_count:7}},'play');assert.equal(h.api.boards.practice.analysis,null);
});
test('disposing while IPC is pending prevents late publication',async()=>{
  const h=harness();const pending=h.api.run('analyze');await flush();h.api.dispose();h.complete({analysis:{candidate_count:99}});await pending;assert.equal(h.api.board.analysis,null);
});
