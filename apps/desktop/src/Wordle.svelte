<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import './wordle.css';
  import { bridge, message, type Settings, type WordleCommand, type WordleSnapshot, type WordlePreferences } from './bridge';
  import type { WordleAnalysis, Recommendation } from '../../../contracts/generated/WordleAnalysis';
  import type { HistoryRow, Mark } from '../../../contracts/generated/WordleSnapshot';
  type Mode = 'play' | 'instant' | 'practice';
  type Profile = 'broad' | 'common' | 'custom';
  type Entry = {word:string; feedback:(Mark|null)[]; ready:boolean};
  type FeedbackRow = {word:string; feedback:(Mark|null)[]};
  interface Board {
    length:number; attempts:number; profile:Profile; history:HistoryRow[];
    entries:(Entry|null)[]; activeRow:number;
    draft:string; marks:(Mark|null)[]; cursor:number|null; ready:boolean;
    fixed:string; forbidden:string[]; bounds:string; remaining:number;
    analysis:WordleAnalysis|null; analyzedClues:string|null; error:string;
    finished:boolean; revealed:string|null; started:boolean; hints:number; revision:number;
  }
  let {settings, visible, frequencyAvailable, active = $bindable(false)}: {settings:Settings; visible:boolean; frequencyAvailable:boolean; active?:boolean} = $props();
  let prefs = $state<Required<WordlePreferences>>(untrack(()=>({length:5,attempts:6,mode:'play',play_profile:'broad',instant_profile:'broad',practice_profile:'common',answers_path:'',guesses_path:'',...structuredClone($state.snapshot(settings.wordle))})));
  let mode = $state<Mode>(prefs.mode);
  function make(profile:Profile):Board {
    return {length:prefs.length,attempts:prefs.attempts,profile,history:[],entries:Array(prefs.attempts).fill(null),activeRow:0,draft:'',marks:Array(prefs.length).fill(null),cursor:0,ready:false,fixed:'.'.repeat(prefs.length),forbidden:Array(prefs.length).fill(''),bounds:'',remaining:prefs.attempts,analysis:null,analyzedClues:null,error:'',finished:false,revealed:null,started:false,hints:0,revision:0};
  }
  let boards = $state<Record<Mode,Board>>({play:make(prefs.play_profile),instant:make(prefs.instant_profile),practice:make(untrack(()=>frequencyAvailable)?prefs.practice_profile:'broad')});
  let b = $derived(boards[mode]);
  const marks:(Mark|null)[] = [null,'gray','yellow','green'];
  const names = {play:'Play Along',instant:'Instant Solver',practice:'Practice'};
  const profiles = {broad:'Broad English',common:'Common English',custom:'Custom lists'};
  const percent = (n:number) => `${(100*n).toLocaleString(undefined,{maximumSignificantDigits:3})}%`;
  const validWord = (board:Board) => new RegExp(`^[a-z]{${board.length}}$`,'i').test(board.draft);
  function effectiveRows(board:Board):FeedbackRow[] {
    return Array.from({length:board.attempts},(_,i)=>i===board.activeRow
      ? {word:board.draft,feedback:board.marks,ready:board.ready}
      : board.entries[i]??null)
      .filter((row):row is Entry=>!!row&&new RegExp(`^[a-z]{${board.length}}$`,'i').test(row.word)&&(row.ready||row.feedback.some(mark=>mark!==null)))
      .map(row=>({word:row.word.toLowerCase(),feedback:[...row.feedback]}));
  }
  function effectiveHistory(board:Board):HistoryRow[] {
    return effectiveRows(board).filter(row=>row.feedback.every(mark=>mark!==null)).map(row=>({word:row.word,feedback:row.feedback as Mark[]}));
  }
  const partialHistory = (board:Board) => effectiveRows(board).filter(row=>row.feedback.some(mark=>mark===null));
  function missingFeedback(board:Board):{index:number;word:string}[] {
    return Array.from({length:board.attempts},(_,index)=>{
      const row=index===board.activeRow?{word:board.draft,feedback:board.marks}:board.entries[index];
      return row&&new RegExp(`^[a-z]{${board.length}}$`,'i').test(row.word)&&row.feedback.every(mark=>mark===null)
        ? {index,word:row.word.toUpperCase()}:null;
    }).filter((row):row is {index:number;word:string}=>row!==null);
  }
  let pendingFeedback = $derived(mode==='play'?missingFeedback(b):[]);
  let needsFeedback = $derived(pendingFeedback.length>0);
  const clueKey = (board:Board) => JSON.stringify([board.fixed,board.forbidden,board.bounds,board.remaining,effectiveRows(board)]);
  let stale = $derived(mode!=='practice' && b.analysis!==null && b.analyzedClues!==clueKey(b));
  let marked = $derived(b.marks.filter(mark=>mark!==null).length);
  let recommendations = $derived(b.analysis ? [b.analysis.strategic,b.analysis.answer].filter((r,i,a):r is Recommendation=>!!r&&(i===0||r.word!==a[0]?.word)) : []);
  let keyMarks = $derived.by(()=>{
    const out:Record<string,Mark>={}, rank={gray:0,yellow:1,green:2};
    const rows:FeedbackRow[]=mode==='practice'?b.history:effectiveRows(b);
    for(const row of rows) [...row.word].forEach((letter,i)=>{const key=letter.toUpperCase(),mark=row.feedback[i];if(mark&&(!out[key]||rank[mark]>rank[out[key]]))out[key]=mark;});
    return out;
  });
  let boardArea = $state<HTMLDivElement>();
  let showCandidates = $state(false), setupOpen = $state(false);
  let search = $state(''), sort = $state('frequency'), scrollTop = $state(0), candidateHeight = $state(380);
  let candidateViewport = $state<HTMLDivElement>();
  function candidateWords():string[] {
    const words=(b.analysis?.candidates??[]).filter(word=>word.includes(search.trim().toLowerCase()));
    if(sort==='az') words.sort();
    else if(sort==='za') words.sort().reverse();
    else if(sort==='distinct') words.sort((a,c)=>new Set(c).size-new Set(a).size||a.localeCompare(c));
    return words;
  }
  let filtered = $derived.by(candidateWords);
  let firstVisible = $derived(Math.max(0,Math.floor((scrollTop-38)/38)-5));
  let lastVisible = $derived(Math.min(filtered.length,firstVisible+Math.ceil(candidateHeight/38)+12));
  function resetScroll(){scrollTop=0;if(candidateViewport)candidateViewport.scrollTop=0;}
  $effect(()=>{void filtered;untrack(resetScroll);});

  let pendingId=0, pendingMode:Mode='play', pendingClues:string|null=null, pendingAction:WordleCommand['action']='analyze';
  let queued:Mode|null=null, poll:ReturnType<typeof setTimeout>, updateTimer:ReturnType<typeof setTimeout>, disposed=false;
  async function savePrefs(){
    prefs.length=b.length;prefs.attempts=b.attempts;prefs.mode=mode;
    prefs.play_profile=boards.play.profile;prefs.instant_profile=boards.instant.profile;prefs.practice_profile=boards.practice.profile;
    try{await bridge.wordlePreferences($state.snapshot(prefs));settings.wordle=$state.snapshot(prefs);}catch(e){b.error=message(e);}
  }
  async function switchMode(next:Mode){
    mode=next;showCandidates=false;search='';sort='frequency';await savePrefs();
    if(next!=='practice' && (!b.analysis||stale)) await run('analyze');
  }
  function constraints(board:Board=b,target:Mode=mode){
    if(target!=='instant')return {fixed:[],forbidden_positions:[],counts:[]};
    if(board.fixed.length!==board.length||!/^[a-z.]+$/i.test(board.fixed))throw new Error(`Fixed positions need ${board.length} letters or empty positions.`);
    const counts=board.bounds.split('\n').filter(line=>line.trim()).map(line=>{
      const fields=line.trim().split(/\s+/);
      if(fields.length<2||fields.length>3||!/^[a-z]$/i.test(fields[0])||!fields.slice(1).every(n=>/^\d+$/.test(n)))throw new Error('Letter counts: one letter, minimum, optional maximum per line.');
      return {letter:fields[0],min:Number(fields[1]),max:fields[2]===undefined?null:Number(fields[2])};
    });
    return {fixed:[...board.fixed].map(ch=>ch==='.'?null:ch.toLowerCase()),forbidden_positions:board.forbidden,counts};
  }
  function drain(){if(!disposed&&!active&&queued){const target=queued;queued=null;void run('analyze',target);}}
  async function run(action:WordleCommand['action'],target:Mode=mode){
    if(disposed)return;
    if(active){if(action==='analyze'){queued=target;void bridge.wordleCancel(pendingId).catch(()=>{});}return;}
    const board=boards[target];board.error='';
    try{
      const history=target==='practice'?$state.snapshot(board.history):effectiveHistory(board);
      const partial_history=target==='practice'?[]:partialHistory(board);
      const finished=history.some(row=>row.feedback.every(mark=>mark==='green'));
      const command:WordleCommand={mode:target,action,revision:board.revision+1,profile:board.profile,attempts:board.attempts,answers_path:prefs.answers_path,guesses_path:prefs.guesses_path,word:board.draft,request:{length:board.length,attempts_remaining:target==='instant'?board.remaining:finished?0:Math.max(0,board.attempts-history.length-partial_history.length),history,partial_history,constraints:constraints(board,target),offset:0,limit:100}};
      board.revision++;active=true;pendingMode=target;pendingClues=clueKey(board);pendingAction=action;
      pendingId=await bridge.wordleStart(command);await refresh();
    }catch(e){board.error=message(e);active=false;drain();}
  }
  function accept(snapshot:WordleSnapshot,target:Mode,analyzedClues?:string|null){
    const board=boards[target];if(snapshot.revision!==board.revision)return;
    if(snapshot.analysis&&!snapshot.error){board.analysis=snapshot.analysis;if(analyzedClues!==undefined)board.analyzedClues=analyzedClues;}
    if(snapshot.error)board.error=snapshot.error;
    if(target==='practice'&&(snapshot.analysis||snapshot.error)){
      board.history=snapshot.history;board.finished=snapshot.finished;board.revealed=snapshot.revealed_answer??null;board.started=!!snapshot.analysis;
    }
  }
  async function refresh(){
    if(disposed)return;
    try{
      const snapshot=await bridge.wordleStatus();if(disposed||snapshot.id!==pendingId)return;
      accept(snapshot,pendingMode,pendingClues);active=snapshot.active;
      if(active)poll=setTimeout(refresh,60);
      else{
        if(!snapshot.error&&pendingMode==='practice'){const board=boards.practice;board.draft='';board.cursor=0;board.marks=Array(board.length).fill(null);board.ready=false;if(pendingAction==='guess')board.hints=0;if(mode==='practice')focusBoard();}
        drain();
      }
    }catch(e){boards[pendingMode].error=message(e);active=false;drain();}
  }
  function changed(){
    const key=clueKey(b);
    b.error='';
    if((active&&pendingMode===mode&&pendingClues===key)||(!active&&b.analysis&&b.analyzedClues===key))return;
    b.revision++;b.error='';queued=mode;clearTimeout(updateTimer);
    if(active)void bridge.wordleCancel(pendingId).catch(()=>{});
    updateTimer=setTimeout(drain,90);
  }
  async function reset(){
    if(active)return;clearTimeout(updateTimer);queued=null;
    const previous=b;
    boards[mode]={...make(previous.profile),length:previous.length,attempts:previous.attempts,entries:Array(previous.attempts).fill(null),marks:Array(previous.length).fill(null),fixed:'.'.repeat(previous.length),forbidden:Array(previous.length).fill(''),remaining:previous.attempts,revision:previous.revision};
    showCandidates=false;search='';setupOpen=false;await savePrefs();if(mode!=='practice')await run('analyze');focusBoard();
  }
  function focusBoard(){requestAnimationFrame(()=>boardArea?.focus({preventScroll:true}));}
  function changeDraft(value:string){b.draft=value.toUpperCase();b.marks=Array(b.length).fill(null);b.ready=false;b.error='';if(mode!=='practice')changed();}
  function typeKey(key:string){
    if(mode==='instant'||(mode==='practice'&&(b.finished||!b.started||active)))return;
    if(key==='ENTER'){void submit();return;}
    if(key==='LEFT'||key==='RIGHT'){b.cursor=Math.max(0,Math.min(b.length-1,(b.cursor??b.length)+(key==='LEFT'?-1:1)));return;}
    if((key==='UP'||key==='DOWN')&&mode==='play'){selectRow(Math.max(0,Math.min(b.attempts-1,b.activeRow+(key==='UP'?-1:1))));return;}
    if(key==='BACKSPACE'){
      const last=b.draft.trimEnd().length-1;
      if(last>=0){changeDraft(b.draft.slice(0,last).trimEnd());b.cursor=last;}
      return;
    }
    if(key==='DELETE'){
      if(b.cursor!==null&&b.cursor<b.draft.length)changeDraft((b.draft.slice(0,b.cursor)+' '+b.draft.slice(b.cursor+1)).trimEnd());
      return;
    }
    if(/^[a-z]$/i.test(key)){
      const hole=b.draft.indexOf(' '),i=b.cursor??(hole>=0?hole:b.draft.length);if(i>=b.length)return;
      const letters=b.draft.padEnd(b.length,' ').split('');letters[i]=key.toUpperCase();
      changeDraft(letters.join('').trimEnd());b.cursor=i+1<b.length?i+1:null;
    }
  }
  function editRow(index:number){
    selectRow(index);
  }
  function saveRow(finish=false){
    if(!Number.isInteger(b.activeRow)||b.activeRow<0||b.activeRow>=b.attempts)return;
    b.ready=b.ready||(finish&&validWord(b));
    b.entries[b.activeRow]=b.draft.trim()?{word:b.draft,feedback:[...b.marks],ready:b.ready}:null;
  }
  function selectRow(index:number){
    if(mode==='practice'||!Number.isInteger(index)||index<0||index>=b.attempts)return;
    if(index!==b.activeRow){
      saveRow(true);b.activeRow=index;
      const row=b.entries[index];b.draft=row?.word??'';b.marks=row?[...row.feedback]:Array(b.length).fill(null);b.ready=row?.ready??false;
      b.cursor=0;changed();
    }
    focusBoard();
  }
  function setFeedback(mark:Mark|null){if(mode==='practice'||!validWord(b))return;b.ready=true;b.marks[b.cursor??0]=mark;changed();focusBoard();}
  function tileClick(row:number,column:number,direction=1){
    if(!Number.isInteger(row)||row<0||row>=b.attempts||!Number.isInteger(column)||column<0||column>=b.length)return;
    if(mode==='practice'){if(row!==b.history.length||active||b.finished)return;b.cursor=column;focusBoard();return;}
    if(row!==b.activeRow){selectRow(row);b.cursor=column;return;}
    b.cursor=column;
    if(validWord(b)){const index=marks.indexOf(b.marks[column]);setFeedback(marks[(index+direction+marks.length)%marks.length]);}focusBoard();
  }
  async function removeRow(){
    if(mode==='practice')return;b.entries[b.activeRow]=null;b.draft='';b.marks=Array(b.length).fill(null);b.cursor=0;b.ready=false;b.finished=false;changed();focusBoard();
  }
  async function submit(){
    if(mode==='practice'&&b.finished)return;
    if(!validWord(b)){b.error=`Enter ${b.length} English letters.`;return;}
    if(mode==='practice'){await run('guess');return;}
    if(!b.ready){b.ready=true;saveRow();changed();focusBoard();return;}
    commitFeedback();focusBoard();
  }
  function commitFeedback(){
    if(!validWord(b))return false;
    saveRow(true);selectRow(Math.min(b.attempts-1,b.activeRow+1));changed();return true;
  }
  function useWord(word:string){
    if(mode==='instant')return;
    // A live recommendation follows the completed row. Preserve that feedback
    // before filling the next row; never replace the clues that produced it.
    if(validWord(b)&&b.ready){
      const next=Array.from({length:b.attempts},(_,i)=>i).find(i=>i!==b.activeRow&&!b.entries[i]);
      if(next===undefined)return;saveRow(true);selectRow(next);
    }
    changeDraft(word);b.cursor=0;focusBoard();
  }
  function nextHint(){b.hints++;if(b.hints>=2)void run('analyze');}
  async function choose(which:'answers_path'|'guesses_path'){
    try{const path=await bridge.choose('dictionary');if(path){prefs[which]=path;await savePrefs();}}catch(e){b.error=message(e);}
  }
  function setFixed(index:number,value:string){const ch=value.replace(/[^a-z]/gi,'').slice(-1).toLowerCase()||'.';b.fixed=b.fixed.slice(0,index)+ch+b.fixed.slice(index+1);changed();}
  function keydown(event:KeyboardEvent){
    if(!visible||mode==='instant'||event.ctrlKey||event.altKey||event.metaKey||document.querySelector('dialog[open]'))return;
    const target=event.target as HTMLElement;if(target.closest('input,textarea,select,summary'))return;
    if(event.key==='Enter'&&target.closest('button'))return;
    const aliases:Record<string,string>={Enter:'ENTER',Backspace:'BACKSPACE',Delete:'DELETE',ArrowLeft:'LEFT',ArrowRight:'RIGHT',ArrowUp:'UP',ArrowDown:'DOWN'};
    if(aliases[event.key]||/^[a-z]$/i.test(event.key)){event.preventDefault();typeKey(aliases[event.key]??event.key);}
  }
  onMount(()=>{document.addEventListener('keydown',keydown);return()=>{disposed=true;clearTimeout(poll);clearTimeout(updateTimer);document.removeEventListener('keydown',keydown);};});
  let opened=false;
  $effect(()=>{if(visible&&!opened){opened=true;untrack(()=>{if(mode!=='practice')void run('analyze');});}});
</script>

{#snippet explanation(rec:Recommendation)}
  <dl class="wordle-metrics"><div><dt>Information gain</dt><dd>{rec.information_bits.toFixed(2)} bits</dd></div><div><dt>Expected answers left</dt><dd>{rec.expected_remaining.toFixed(1)}</dd></div><div><dt>Immediate win</dt><dd>{percent(rec.immediate_win_probability)}</dd></div>{#if rec.solve_probability!=null}<div><dt>Solve within attempts</dt><dd>{percent(rec.solve_probability)}</dd></div>{/if}</dl>
{/snippet}
{#snippet candidates()}
  <div class="candidate-tools"><label>Search words<input type="search" bind:value={search} placeholder="Contains letters…"/></label><label>Sort<select bind:value={sort}><option value="frequency">{b.analysis?.profile.frequency_available?'Frequency':'Dictionary order'}</option><option value="az">A to Z</option><option value="za">Z to A</option><option value="distinct">Different letters</option></select></label></div>
  <p class="hint">{filtered.length.toLocaleString()} {search?'matching':'remaining'} words{sort==='frequency'?(b.analysis?.profile.frequency_available?' · corpus frequency order':' · frequency data unavailable'):''}</p>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex (The scroll region must support keyboard scrolling.) -->
  <div class="candidate-scroll" bind:this={candidateViewport} bind:clientHeight={candidateHeight} onscroll={event=>scrollTop=event.currentTarget.scrollTop} role="region" aria-label="Candidate words" tabindex="0">
    <table aria-rowcount={filtered.length+1}><thead><tr><th scope="col">Word</th><th scope="col">Different letters</th>{#if mode!=='instant'}<th scope="col"><span class="sr-only">Action</span></th>{/if}</tr></thead><tbody>
      {#if firstVisible>0}<tr aria-hidden="true" class="candidate-spacer"><td colspan={mode==='instant'?2:3} style={`height:${Math.min(firstVisible,filtered.length)*38}px`}></td></tr>{/if}
      {#each filtered.slice(firstVisible,lastVisible) as word,i}<tr aria-rowindex={firstVisible+i+2}><td class="candidate-word">{word.toUpperCase()}</td><td>{new Set(word).size}</td>{#if mode!=='instant'}<td><button disabled={stale||b.finished||needsFeedback} onclick={()=>useWord(word)} aria-label={`Use ${word.toUpperCase()}`}>Use</button></td>{/if}</tr>{/each}
      {#if lastVisible<filtered.length}<tr aria-hidden="true" class="candidate-spacer"><td colspan={mode==='instant'?2:3} style={`height:${(filtered.length-lastVisible)*38}px`}></td></tr>{/if}
    </tbody></table>{#if filtered.length===0}<p class="candidate-empty">{search?'No words match this search.':'No matching words.'}</p>{/if}
  </div>
{/snippet}

<section class="wordle-shell" aria-label="Wordle assistant">
  <div class="wordle-heading"><div><h2>Wordle</h2><p class="muted">{mode==='play'?'Your puzzle, with a little help.':mode==='instant'?'Find words from the clues you know.':'A local puzzle. Help when you want it.'}</p></div><nav class="wordle-modes" aria-label="Wordle modes">{#each ['play','instant','practice'] as value}<button aria-pressed={mode===value} onclick={()=>switchMode(value as Mode)}>{names[value as Mode]}</button>{/each}</nav></div>
  <div class="wordle-toolbar">
    <details class="wordle-setup" bind:open={setupOpen}><summary>{b.length} letters <span>·</span> {b.attempts} attempts <span>·</span> {profiles[b.profile]}</summary><div class="wordle-config">
      <label>Word length<select disabled={active} value={b.length} onchange={event=>{b.length=Number(event.currentTarget.value);void reset();}}>{#each [4,5,6,7,8] as length}<option value={length}>{length} letters</option>{/each}</select></label>
      <label>Attempts<select disabled={active} value={b.attempts} onchange={event=>{b.attempts=Number(event.currentTarget.value);void reset();}}>{#each Array.from({length:12},(_,i)=>i+1) as attempts}<option value={attempts}>{attempts}</option>{/each}</select></label>
      <label>Vocabulary<select disabled={active} value={b.profile} onchange={event=>{b.profile=event.currentTarget.value as Profile;void reset();}}><option value="broad">Broad English</option><option value="common">Common English · frequency-derived</option><option value="custom">Custom lists</option></select></label><p class="hint">Changing these options starts a new puzzle.</p>
    </div></details><button disabled={active} onclick={reset}>+ New {mode==='practice'?'game':'puzzle'}</button>
  </div>
  {#if b.profile==='custom'}<div class="wordle-custom"><label>Answer list<div class="path-field"><input readonly value={prefs.answers_path}/><button disabled={active} onclick={()=>choose('answers_path')}>Choose…</button></div></label><label>Allowed guesses <span>(optional)</span><div class="path-field"><input readonly value={prefs.guesses_path}/><button disabled={active} onclick={()=>choose('guesses_path')}>Choose…</button></div></label></div>{/if}
  {#if b.error}<div class="error" role="alert">{b.error}</div>{/if}
  {#if mode==='practice'&&!b.started}
    <div class="practice-start"><div class="practice-preview" aria-hidden="true">{#each [...'READY'] as letter}<span>{letter}</span>{/each}</div><h3>Ready for a new puzzle?</h3><p>{b.length} letters. {b.attempts} guesses. Feedback is automatic.</p><button class="primary" disabled={active} onclick={()=>{void run('new_practice');focusBoard();}}>Start game</button><p class="hint">Use the keyboard or type directly into the board. Hints stay hidden until you ask.</p></div>
  {:else}
    <div class="wordle-columns" class:instant={mode==='instant'}>
      {#if mode==='instant'}
        <form class="wordle-constraints" onsubmit={event=>{event.preventDefault();void run('analyze');}}>
          <h3>Known positions</h3><p class="hint">Put a letter in each position you know.</p>
          <div class="constraint-positions" style={`--letters:${b.length}`}>{#each Array.from({length:b.length},(_,i)=>i) as i}<label><span>{i+1}</span><input aria-label={`Fixed letter at position ${i+1}`} value={b.fixed[i]==='.'?'':b.fixed[i]} oninput={event=>setFixed(i,event.currentTarget.value)} maxlength="1" placeholder="·" autocomplete="off" spellcheck="false"/></label>{/each}</div>
          <h3>Excluded from positions</h3><div class="constraint-positions forbidden" style={`--letters:${b.length}`}>{#each b.forbidden as _,i}<label><span>{i+1}</span><input value={b.forbidden[i]} oninput={event=>{b.forbidden[i]=event.currentTarget.value;changed();}} aria-label={`Forbidden letters at position ${i+1}`} spellcheck="false"/></label>{/each}</div>
          <label>Letter counts<textarea rows="3" value={b.bounds} oninput={event=>{b.bounds=event.currentTarget.value;changed();}} placeholder={'e 1 2\ns 0 0'}></textarea><span class="hint">Letter, minimum, optional maximum. “s 0 0” excludes S.</span></label>
          <label>Attempts remaining<input type="number" min="0" max="12" value={b.remaining} oninput={event=>{b.remaining=Number(event.currentTarget.value);changed();}}/></label><button class="primary" type="submit">Update matches</button>
          <details class="instant-history"><summary>Previous feedback ({effectiveRows(b).length})</summary>
            {#each b.entries as row,i}{#if row}<button type="button" class="history-word" onclick={()=>editRow(i)}>Row {i+1}: {row.word.toUpperCase()} <span class="hint">Edit</span></button>{/if}{/each}
            <label>Guess in row {b.activeRow+1}<input value={b.draft} oninput={event=>changeDraft(event.currentTarget.value)} maxlength={b.length} spellcheck="false" autocomplete="off"/></label>
            <div class="instant-feedback" style={`--letters:${b.length}`}>{#each b.marks as mark,i}<button type="button" class={`wordle-tile ${mark??''}`} disabled={!validWord(b)} aria-label={`Feedback at position ${i+1}: ${mark??'unset'}`} onclick={()=>{b.cursor=i;setFeedback(marks[(marks.indexOf(mark)+1)%marks.length]);}} oncontextmenu={event=>{event.preventDefault();b.cursor=i;setFeedback(marks[(marks.indexOf(mark)+marks.length-1)%marks.length]);}}>{b.draft[i]?.toUpperCase()??''}</button>{/each}</div>
            <p class="hint">Known feedback updates matches immediately. Unset tiles are ignored.</p><div class="actions"><button type="button" disabled={!validWord(b)} onclick={()=>{commitFeedback();}}>Next row</button>{#if b.draft}<button type="button" onclick={removeRow}>Clear row</button>{/if}</div>
          </details>
        </form>
      {:else}
        <div class="wordle-game"><div class="board-heading"><h3>Your board</h3><span class="muted">{mode==='practice'?b.history.length:effectiveRows(b).length} / {b.attempts} guesses</span></div>
          <div class="board-stage" class:practice={mode==='practice'} style={`--board-width:calc(${b.length} * var(--tile-size) + ${(b.length-1)*6}px)`}>
            {#if mode==='play'}<div class="feedback-palette" role="group" aria-label="Feedback for selected tile"><span class="hint">Feedback</span>{#each marks as mark}<button class={mark??'unset'} aria-pressed={b.marks[b.cursor??0]===mark} disabled={!validWord(b)} onclick={()=>setFeedback(mark)}>{mark==='green'?'● Green':mark==='yellow'?'◆ Yellow':mark==='gray'?'× Gray':'Unset'}</button>{/each}</div>{/if}
            <div class="wordle-board" role="grid" aria-rowcount={b.attempts} aria-colcount={b.length} tabindex="0" bind:this={boardArea} aria-label="Wordle board. Type letters, Delete clears the selected tile, Backspace removes the last letter, Enter finishes the row.">
              {#each Array.from({length:b.attempts},(_,i)=>i) as rowIndex}
                {@const old=mode==='practice'?b.history[rowIndex]:b.entries[rowIndex]}{@const draft=mode==='practice'?rowIndex===b.history.length&&!b.finished:rowIndex===b.activeRow}
                <div class="wordle-row" role="row" class:active-row={draft} style={`--letters:${b.length}`}>
                  {#each Array.from({length:b.length},(_,i)=>i) as i}{@const letter=draft?b.draft[i]?.trim():old?.word[i]}{@const mark=draft?b.marks[i]:old?.feedback[i]}
                    <button role="gridcell" aria-selected={draft&&i===b.cursor} class={`wordle-tile ${mark??''}`} class:selected={draft&&i===b.cursor} disabled={mode==='practice'&&(!draft||active||b.finished)} aria-label={`Row ${rowIndex+1}, position ${i+1}: ${letter?.toUpperCase()||'empty'}, ${mark??'unset'}`} onclick={()=>tileClick(rowIndex,i)} oncontextmenu={event=>{event.preventDefault();if(mode==='play')tileClick(rowIndex,i,-1);}}>{letter?.toUpperCase()??''}<span aria-hidden="true">{mark==='green'?'●':mark==='yellow'?'◆':mark==='gray'?'×':''}</span></button>
                  {/each}
                </div>
              {/each}
            </div>
            <p class="board-help" aria-live="polite">{mode==='practice'?(b.finished?`Answer: ${b.revealed?.toUpperCase()??''}`:'Type a word. Press Enter to guess.'):b.ready?`${marked} of ${b.length} marked. Feedback is live; click another row to continue.`:'Type into the board, mark known feedback, and select any row. Changes are saved.'}</p>
            <div class="wordle-keyboard" role="group" aria-label="On-screen keyboard">{#each ['QWERTYUIOP','ASDFGHJKL','ZXCVBNM'] as row}<div>{#if row==='ZXCVBNM'}<button onclick={()=>{typeKey('ENTER');focusBoard();}}>Enter</button>{/if}{#each [...row] as key}<button class={keyMarks[key]??''} onclick={()=>{typeKey(key);focusBoard();}} aria-label={`${key}${keyMarks[key]?`, ${keyMarks[key]}`:''}`}>{key}</button>{/each}{#if row==='ZXCVBNM'}<button onclick={()=>{typeKey('BACKSPACE');focusBoard();}} aria-label="Backspace">⌫</button>{/if}</div>{/each}</div>
            {#if mode==='play'}<div class="board-actions">{#if b.draft}<button onclick={removeRow}>Clear row</button>{/if}<button disabled={b.activeRow===0} onclick={()=>selectRow(b.activeRow-1)}>Previous row</button><button disabled={b.activeRow===b.attempts-1} onclick={()=>selectRow(b.activeRow+1)}>Next row</button></div>{/if}
          </div>
        </div>
      {/if}
      <div class="wordle-assistant" class:instant-results={mode==='instant'} aria-busy={active}>
        <div class="assistant-heading"><h3>{mode==='instant'?'Matching words':'Assistant'}</h3>{#if active}<span class="hint" role="status">Updating…</span>{/if}</div>
        {#if mode==='practice'&&b.hints<2&&!b.finished}
          <div class="practice-help"><h4>{b.history.length?'Keep going':'Make your first guess'}</h4><p>{b.history.length} of {b.attempts} guesses used. Your feedback is on the board.</p>{#if b.hints===1}<p class="general-hint">{b.history.length?'Keep green letters in place, move yellow letters, and try new letters in the remaining positions.':'Start with a familiar word using several different letters and a mix of vowels and consonants.'}</p>{/if}<button class="primary" disabled={active} onclick={nextHint}>{b.hints===0?'Get a hint':'Suggest a word'}</button><p class="hint">{b.hints===0?'First, a little guidance. A word suggestion only if you ask again.':'Ready for a more specific hint?'}</p></div>
        {:else}
          {#if needsFeedback}<div class="wordle-feedback-needed" role="status"><h4>Add feedback to get your next suggestion</h4><p>These words have no feedback yet. Typing a word does not eliminate answers. Mark any known tiles; unset tiles stay unknown.</p><div class="wordle-missing-words">{#each pendingFeedback as row}<button onclick={()=>selectRow(row.index)}>Row {row.index+1}: {row.word}</button>{/each}</div></div>{/if}
          {#if stale}<p class="wordle-stale" role="status">{active?'Updating the assistant…':'Results are from the previous clues.'}</p>{/if}
          {#if b.analysis}
            {#if b.analysis.diagnostic}<div class="error">{b.analysis.diagnostic==='contradictory_clues'?'The clues conflict. Check positions, counts, and feedback.':'No words match this profile. Check the feedback or choose a broader vocabulary.'}</div>{/if}
            {#if recommendations[0]&&mode!=='instant'&&!needsFeedback}{@const rec=recommendations[0]}<article class="wordle-primary"><p class="muted">Suggested guess</p><div class="suggested-tiles" aria-label={rec.word.toUpperCase()}>{#each [...rec.word] as letter}<span aria-hidden="true">{letter.toUpperCase()}</span>{/each}</div><button class="primary" disabled={stale||b.finished||active} onclick={()=>useWord(rec.word)}>Use word</button><p class="hint">{rec.immediate_win_probability>0?'Also a possible answer.':'Explore letters to distinguish the remaining answers.'}</p></article>{/if}
            <p class="wordle-count"><strong>{b.analysis.candidate_count.toLocaleString()}</strong> possible {b.analysis.candidate_count===1?'answer':'answers'}</p><p class="hint">{b.analysis.profile.name} · equally likely answers · {b.analysis.attempts_remaining} attempts left</p>
            {#each b.analysis.notices as notice}<p class="hint">{notice}</p>{/each}
            {#if mode==='instant'}{@render candidates()}{:else}<details class="candidate-details" bind:open={showCandidates}><summary>Remaining words</summary>{#if showCandidates}{@render candidates()}{/if}</details>{/if}
            <details class="wordle-details"><summary>{needsFeedback?'Vocabulary and search':'Detailed guess analysis'}</summary>{#if !needsFeedback}<p class="hint">{b.analysis.optimal_for_profile?'Optimal for this profile':b.analysis.method==='no_next_guess'?'No next guess':b.analysis.method==='two_turn_search'?'Two-attempt search':'Heuristic suggestion'}</p>{#each recommendations as rec,i}<div class="analysis-guess"><h4>{i===0?'Strategic guess':'Possible answer'}: {rec.word.toUpperCase()}</h4>{@render explanation(rec)}</div>{/each}{:else}<p class="hint">Waiting for feedback; no new guess is recommended yet.</p>{/if}<p class="hint path">Vocabulary: {b.analysis.profile.fingerprint}</p><p class="hint">{b.analysis.evaluated_guesses} guesses evaluated · {b.analysis.guess_space_complete?'full guess space covered':'guess space partly evaluated'}<br/>{b.analysis.expanded_states} search states · ranking {b.analysis.recommendation_ms.toFixed(0)} ms<br/>{b.analysis.profile.frequency_available?'Corpus frequency breaks ties.':'Frequency data is unavailable.'} Answer probabilities are uniform.</p></details>
          {:else}<p class="muted">{active?'Finding possible answers…':'Enter the clues you know to find matching words.'}</p>{/if}
        {/if}
        {#if mode==='practice'&&b.started&&!b.finished}<details class="wordle-details"><summary>Finish this game</summary><button disabled={active} onclick={()=>run('reveal')}>Reveal answer</button></details>{/if}
      </div>
    </div>
  {/if}
</section>
