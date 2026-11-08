// Exercise the component's actual input/session code with controlled IPC timing.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const {test}=require('node:test');
const ts=require('../apps/desktop/node_modules/typescript');
function deferred(){let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};}
const flush=()=>new Promise(setImmediate);
function harness(mode='play'){
  const commands=[],statuses=[],cancelled=[],timers=new Map(),frames=[];let timerId=0;
  const document={activeElement:null,body:{control:'body'},querySelector:()=>null};
  const boardArea={contains:target=>target===boardArea||target?.gridCell===true,focus(){document.activeElement=boardArea;},querySelector:()=>null};
  const props=Object.assign(()=>({settings:{wordle:preferences},visible:true,frequencyAvailable:false}),{id:()=> 'test-wordle'});
  const preferences={length:5,attempts:6,mode,play_profile:'broad',instant_profile:'broad',practice_profile:'broad',answers_path:'',guesses_path:''};
  const source=fs.readFileSync(path.join(__dirname,'../apps/desktop/src/Wordle.svelte'),'utf8').split('<script lang="ts">')[1].split('</script>')[0].replace(/^\s*import .*;$/gm,'');
  const state=Object.assign(x=>x,{snapshot:structuredClone}),derived=Object.assign(x=>x,{by:fn=>fn()});
  const context=vm.createContext({$state:state,$derived:derived,$bindable:x=>x,$props:props,$effect:()=>{},untrack:fn=>fn(),onMount:()=>{},structuredClone,document,requestAnimationFrame:fn=>frames.push(fn),setTimeout:(fn,ms)=>{timers.set(++timerId,{fn,ms});return timerId;},clearTimeout:id=>timers.delete(id),bridge:{wordleStart:async command=>{commands.push(structuredClone(command));statuses.push(deferred());return commands.length;},wordleStatus:()=>statuses.at(-1).promise,wordleCancel:async id=>{cancelled.push(id);},wordlePreferences:async()=>{}},message:String});
  const expose=`globalThis.api={attachBoard(element){boardArea=element},setActive(value){active=value},setVisible(value){visible=value},setMode(value){mode=value;b=boards[mode]},focusBoard,accept,run,submit,typeKey,keydown,changeDraft,useWord,clueKey,effectiveHistory,effectiveRows,partialHistory,missingFeedback,tileClick,setFeedback,editRow,selectRow,removeRow,nextHint,drain,get board(){return b},get boards(){return boards},get active(){return active},view(query,order){search=query;sort=order;return candidateWords()},dispose(){disposed=true}};`;
  vm.runInContext(ts.transpileModule(source+'\n'+expose,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText,context);
  context.api.attachBoard(boardArea);
  return {api:context.api,commands,statuses,cancelled,document,boardArea,
    frame(){while(frames.length)frames.shift()();},
    key(key,options={}){const event={key,target:boardArea,defaultPrevented:false,preventDefault(){this.defaultPrevented=true;},...options};context.api.keydown(event);return event;},
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
test('entered words without feedback request guidance instead of presenting a new recommendation',async()=>{
  const h=harness();assert.equal(h.api.missingFeedback(h.api.board).length,0);
  const words=['TESTS','HELLO','MIAMI','LARES','PARES'];
  for(let i=0;i<words.length;i++){h.api.selectRow(i);enterWord(h,words[i]);}
  await h.api.submit();
  assert.deepEqual(Array.from(h.api.missingFeedback(h.api.board),row=>row.word),words);
  h.update();await flush();assert.equal(h.commands[0].request.attempts_remaining,1);
  assert.equal(h.commands[0].request.partial_history.length,5);
  for(let i=0;i<words.length;i++){h.api.selectRow(i);h.api.tileClick(i,0);}
  assert.equal(h.api.missingFeedback(h.api.board).length,0);
});
test('an incomplete word or a partially marked word does not block partial-feedback analysis',()=>{
  const h=harness();enterWord(h,'PARE');assert.equal(h.api.missingFeedback(h.api.board).length,0);
  h.api.typeKey('S');assert.equal(h.api.missingFeedback(h.api.board).length,1);
  h.api.tileClick(0,0);assert.equal(h.api.missingFeedback(h.api.board).length,0);
  assert.equal(h.api.partialHistory(h.api.board).length,1);
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
test('physical keys only edit a focused grid in Play Along and Practice',()=>{
  for(const mode of ['play','practice']){
    const h=harness(mode);h.api.board.started=true;
    for(const control of ['button','summary','input','textarea','select','a','scroll region','contenteditable','page body']){
      const before=JSON.stringify(h.api.board);
      for(const key of ['a','ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','Backspace','Delete','Enter',' ']){
        assert.equal(h.key(key,{target:{control}}).defaultPrevented,false,`${mode}: ${control} / ${key}`);
      }
      assert.equal(JSON.stringify(h.api.board),before);
    }
    assert.equal(h.key('a').defaultPrevented,true);assert.equal(h.api.board.draft,'A');
  }
});
test('Instant Solver, hidden boards, dialogs, modifiers and composition keep native keys',()=>{
  const h=harness();
  for(const options of [{ctrlKey:true},{altKey:true},{metaKey:true},{isComposing:true},{defaultPrevented:true}]){
    h.key('a',options);assert.equal(h.api.board.draft,'');
  }
  h.api.setVisible(false);assert.equal(h.key('a').defaultPrevented,false);h.api.setVisible(true);
  h.document.querySelector=()=>({open:true});assert.equal(h.key('a').defaultPrevented,false);h.document.querySelector=()=>null;
  h.api.setMode('instant');assert.equal(h.key('a').defaultPrevented,false);assert.equal(h.api.board.draft,'');
  h.api.setMode('play');h.key('A',{shiftKey:true});assert.equal(h.api.board.draft,'A');
  assert.equal(h.key('Tab').defaultPrevented,false);assert.equal(h.key('Escape').defaultPrevented,false);
});
test('grid navigation preserves column, drafts and partial feedback on a large board',()=>{
  const h=harness();h.api.board.length=8;h.api.board.attempts=12;h.api.board.marks=Array(8).fill(null);
  enterWord(h,'ABCDEFGH');h.key('End');assert.equal(h.api.board.cursor,7);
  h.key('ArrowRight');assert.equal(h.api.board.cursor,7);
  h.key(' ');assert.equal(h.api.board.marks[7],'gray');
  for(let row=1;row<12;row++){h.key('ArrowDown');assert.equal(h.api.board.activeRow,row);assert.equal(h.api.board.cursor,7);}
  h.key('ArrowDown');assert.equal(h.api.board.activeRow,11);
  for(let row=10;row>=0;row--)h.key('ArrowUp');
  assert.equal(h.api.board.draft,'ABCDEFGH');assert.equal(h.api.board.marks[7],'gray');
  assert.equal(h.api.board.marks[0],null);assert.equal(h.api.partialHistory(h.api.board).length,1);
  h.key('Home');h.key('ArrowLeft');assert.equal(h.api.board.cursor,0);
  h.key('ArrowRight');h.key('Delete');assert.equal(h.api.board.draft,'A CDEFGH');
  h.key('X');assert.equal(h.api.board.draft,'AXCDEFGH');assert.ok(h.api.board.marks.every(mark=>mark===null));
});
test('Space cycles feedback at the selected cell, Shift+Space reverses and Enter finishes the row',async()=>{
  const h=harness();enterWord(h);h.key('Home');h.key('ArrowRight');
  for(const expected of ['gray','yellow','green',null]){assert.equal(h.key(' ').defaultPrevented,true);assert.equal(h.api.board.marks[1],expected);}
  h.key(' ',{shiftKey:true});assert.equal(h.api.board.marks[1],'green');assert.equal(h.api.board.marks[0],null);
  h.key('Enter');await flush();assert.equal(h.api.board.activeRow,1);assert.equal(h.api.board.entries[0].feedback[1],'green');
  h.key(' ');assert.ok(h.api.board.marks.every(mark=>mark===null));
});
test('Practice keeps automatic feedback and ignores input before start, while busy and after finish',()=>{
  const h=harness('practice');assert.equal(h.key('a').defaultPrevented,false);
  h.api.board.started=true;enterWord(h);const before=JSON.stringify(h.api.board);
  assert.equal(h.key(' ').defaultPrevented,false);h.key('ArrowUp');assert.equal(JSON.stringify(h.api.board),before);
  h.api.setActive(true);assert.equal(h.key('Delete').defaultPrevented,false);h.api.setActive(false);
  h.api.board.finished=true;assert.equal(h.key('Delete').defaultPrevented,false);
});
test('grid wiring has one tab stop and exposes the active cell without a global listener',()=>{
  const source=fs.readFileSync(path.join(__dirname,'../apps/desktop/src/Wordle.svelte'),'utf8');
  assert.match(source,/role="grid"[^>]*tabindex="0"[^>]*onkeydown=\{keydown\}[^>]*aria-activedescendant=/);
  assert.match(source,/<button role="gridcell"[^>]*id=\{tileId\(rowIndex,i\)\}[^>]*tabindex="-1"/);
  assert.doesNotMatch(source,/document\.addEventListener\('keydown'/);
});
test('mode switches preserve separate drafts; typing elsewhere does not edit either board',()=>{
  const h=harness();enterWord(h);h.key('Home');h.key(' ');
  h.api.setMode('practice');h.api.board.started=true;h.key('s');
  h.api.setMode('instant');h.key('x',{target:{control:'input'}});
  h.api.setMode('play');assert.equal(h.api.board.draft,'APPLE');assert.equal(h.api.board.marks[0],'gray');
  h.api.setMode('practice');assert.equal(h.api.board.draft,'S');
});
test('Practice guess completion does not pull focus from another control',async()=>{
  const h=harness('practice');h.api.board.started=true;enterWord(h);h.document.activeElement={control:'hint button'};
  const pending=h.api.submit();await flush();h.complete({analysis:{candidate_count:2}});await pending;h.frame();
  assert.equal(h.document.activeElement.control,'hint button');
});
test('Practice start restores focus lost by disabling its button, but respects a new focused control',async()=>{
  for(const moved of [false,true]){
    const h=harness('practice'),start={control:'start button'},other={control:'mode button'};
    h.document.activeElement=start;const pending=h.api.run('new_practice');await flush();
    h.document.activeElement=moved?other:h.document.body;
    h.complete({analysis:{candidate_count:2}});await pending;h.frame();
    assert.equal(h.document.activeElement,moved?other:h.boardArea);
  }
});
test('scheduled board focus never crosses a mode switch, dialog, hidden view or newly focused control',()=>{
  for(const change of [h=>h.api.setMode('practice'),h=>h.api.setVisible(false),h=>h.document.querySelector=()=>({open:true}),h=>h.document.activeElement={control:'another button'}]){
    const h=harness();h.api.focusBoard();change(h);const before=h.document.activeElement;h.frame();assert.equal(h.document.activeElement,before);
  }
});
test('late analysis cannot replace an edited revision or another mode',()=>{
  const h=harness();h.api.board.revision=3;h.api.accept({revision:2,analysis:{candidate_count:99},error:'stale'},'play');
  assert.equal(h.api.board.analysis,null);assert.equal(h.api.board.error,'');h.api.accept({revision:3,analysis:{candidate_count:7}},'play');assert.equal(h.api.boards.practice.analysis,null);
});
test('disposing while IPC is pending prevents late publication',async()=>{
  const h=harness();const pending=h.api.run('analyze');await flush();h.api.dispose();h.complete({analysis:{candidate_count:99}});await pending;assert.equal(h.api.board.analysis,null);
});
