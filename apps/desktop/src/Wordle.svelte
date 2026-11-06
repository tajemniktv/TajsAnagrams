<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import './wordle.css';
  import { bridge, message, type Settings, type WordleCommand, type WordleSnapshot, type WordlePreferences } from './bridge';
  import type { WordleAnalysis, Recommendation } from '../../../contracts/generated/WordleAnalysis';
  import type { HistoryRow, Mark } from '../../../contracts/generated/WordleSnapshot';
  type Mode = 'play' | 'instant' | 'practice';
  type Profile = 'broad' | 'common' | 'custom';
  interface Board { length:number; attempts:number; profile:Profile; history:HistoryRow[]; draft:string; marks:(Mark|null)[]; editing:number|null; fixed:string; forbidden:string[]; bounds:string; remaining:number; analysis:WordleAnalysis|null; analyzedClues:string|null; error:string; finished:boolean; revealed:string|null; started:boolean; assistance:boolean; revision:number; }
  let {settings, visible, frequencyAvailable, active = $bindable(false)}: {settings:Settings; visible:boolean; frequencyAvailable:boolean; active?:boolean} = $props();
  let prefs = $state<Required<WordlePreferences>>(untrack(()=>({length:5,attempts:6,mode:'play',play_profile:'broad',instant_profile:'broad',practice_profile:'common',answers_path:'',guesses_path:'',...structuredClone($state.snapshot(settings.wordle))})));
  let mode = $state<Mode>(prefs.mode);
  function make(profile:Profile):Board {return {length:prefs.length,attempts:prefs.attempts,profile,history:[],draft:'',marks:Array(prefs.length).fill(null),editing:null,fixed:'.'.repeat(prefs.length),forbidden:Array(prefs.length).fill(''),bounds:'',remaining:prefs.attempts,analysis:null,analyzedClues:null,error:'',finished:false,revealed:null,started:false,assistance:false,revision:0};}
  let boards = $state<Record<Mode,Board>>({play:make(prefs.play_profile),instant:make(prefs.instant_profile),practice:make(untrack(()=>frequencyAvailable)?prefs.practice_profile:'broad')});
  let b = $derived(boards[mode]);
  let pendingId = $state(0); let pendingMode = $state<Mode>('play'); let poll:ReturnType<typeof setTimeout>; let disposed=false;
  let showCandidates=$state(false); let historyOpen=$state(false); let selected=$state(0); let guessInput:HTMLInputElement; let pendingClues:string|null=null;
  const marks:(Mark|null)[]=[null,'gray','yellow','green'];
  const names = {play:'Play Along',instant:'Instant Solver',practice:'Practice'};
  const percent = (n:number) => `${(100*n).toLocaleString(undefined,{maximumSignificantDigits:3})}%`;
  const clueKey = (board:Board) => JSON.stringify([board.fixed,board.forbidden,board.bounds,board.remaining,board.history]);
  let stale=$derived(mode==='instant' && b.analysis!==null && b.analyzedClues!==clueKey(b));
  let marked=$derived(b.marks.filter(mark=>mark!==null).length);
  let recommendations=$derived(b.analysis ? [b.analysis.strategic,b.analysis.answer].filter((r,i,a):r is Recommendation => !!r && (i===0 || r.word!==a[0]?.word)) : []);
  let keyMarks=$derived.by(()=>{const out:Record<string,Mark>={};const rank={gray:0,yellow:1,green:2};for(const row of b.history){[...row.word].forEach((letter,i)=>{const key=letter.toUpperCase(), mark=row.feedback[i];if(mark && (!out[key] || rank[mark]>rank[out[key]]))out[key]=mark;});}return out;});

  async function savePrefs() {prefs.length=b.length;prefs.attempts=b.attempts;prefs.mode=mode;prefs.play_profile=boards.play.profile;prefs.instant_profile=boards.instant.profile;prefs.practice_profile=boards.practice.profile;try{await bridge.wordlePreferences($state.snapshot(prefs));settings.wordle=$state.snapshot(prefs);}catch(e){b.error=message(e);}}
  async function switchMode(next:Mode) {mode=next;selected=0;showCandidates=next==='instant';await savePrefs();if(!b.analysis && !active && mode!=='practice')await run('analyze');}
  function constraints() {
    if(mode!=='instant') return {fixed:[],forbidden_positions:[],counts:[]};
    if(b.fixed.length!==b.length || !/^[a-z.]+$/i.test(b.fixed))throw new Error(`Fixed letters need ${b.length} letters or dots.`);
    const counts=b.bounds.split('\n').filter(line=>line.trim()).map(line=>{const fields=line.trim().split(/\s+/);if(fields.length<2 || fields.length>3 || !/^[a-z]$/i.test(fields[0]) || !fields.slice(1).every(n=>/^\d+$/.test(n)))throw new Error('Letter counts: one letter, minimum, optional maximum per line.');return {letter:fields[0],min:Number(fields[1]),max:fields[2]===undefined?null:Number(fields[2])};});
    return {fixed:[...b.fixed].map(ch=>ch==='.'?null:ch.toLowerCase()),forbidden_positions:b.forbidden,counts};
  }
  async function run(action:WordleCommand['action']) {
    if(active)return;
    const target=mode, board=boards[target];board.error='';
    try {
      const command:WordleCommand={mode:target,action,revision:board.revision+1,profile:board.profile,attempts:board.attempts,answers_path:prefs.answers_path,guesses_path:prefs.guesses_path,word:board.draft,request:{length:board.length,attempts_remaining:target==='instant'?board.remaining:Math.max(0,board.attempts-board.history.length),history:$state.snapshot(board.history),constraints:constraints(),offset:0,limit:100}};
      if(target==='play' && board.finished)command.request.attempts_remaining=0;
      board.revision++;active=true;pendingMode=target;pendingClues=target==='instant'?clueKey(board):null;pendingId=await bridge.wordleStart(command);await refresh();
    }catch(e){board.error=message(e);active=false;}
  }
  function accept(snapshot:WordleSnapshot,target:Mode,analyzedClues?:string|null) {
    const board=boards[target];if(snapshot.revision!==board.revision)return;
    if(snapshot.analysis&&!snapshot.error){board.analysis=snapshot.analysis;if(analyzedClues!==undefined)board.analyzedClues=analyzedClues;}
    if(snapshot.error)board.analysis=null;
    if(target==='practice' && (snapshot.analysis||snapshot.error)){board.history=snapshot.history;board.finished=snapshot.finished;board.revealed=snapshot.revealed_answer??null;board.started=!!snapshot.analysis;}
    if(snapshot.error)board.error=snapshot.error;
  }
  async function refresh() {
    if(disposed)return;
    try {const snapshot=await bridge.wordleStatus();if(disposed || snapshot.id!==pendingId)return;accept(snapshot,pendingMode,pendingClues);active=snapshot.active;if(active)poll=setTimeout(refresh,60);else if(!snapshot.error && pendingMode==='practice'){boards.practice.draft='';boards.practice.marks=Array(boards.practice.length).fill(null);}}
    catch(e){boards[pendingMode].error=message(e);active=false;}
  }
  async function reset() {
    if(active)return;
    const previous=b;boards[mode]={...make(previous.profile),length:previous.length,attempts:previous.attempts,marks:Array(previous.length).fill(null),fixed:'.'.repeat(previous.length),forbidden:Array(previous.length).fill(''),remaining:previous.attempts,revision:previous.revision};
    selected=0;historyOpen=false;showCandidates=mode==='instant';await savePrefs();
    await run(mode==='practice'?'new_practice':'analyze');
  }
  function changeDraft(value:string) {b.draft=value.toUpperCase();b.marks=Array(b.length).fill(null);}
  function typeKey(key:string) {if(active || b.finished || (mode==='practice' && !b.started))return;if(mode==='instant')historyOpen=true;if(key==='ENTER'){void submit();return;}if(key==='BACKSPACE'){changeDraft(b.draft.slice(0,-1));return;}if(/^[a-z]$/i.test(key) && b.draft.length<b.length)changeDraft(b.draft+key);}
  function editRow(index:number) {if(mode==='practice'||active)return;const row=b.history[index];b.editing=index;b.draft=row.word.toUpperCase();b.marks=[...row.feedback];selected=0;guessInput?.focus();}
  async function removeRow() {if(b.editing===null)return;b.history.splice(b.editing,1);b.editing=null;b.draft='';b.marks=Array(b.length).fill(null);b.finished=b.history.some(row=>row.feedback.every(m=>m==='green'));await run(mode==='practice'?'filter':'analyze');}
  async function submit() {
    if(active || b.finished && b.editing===null)return;
    if(!new RegExp(`^[a-z]{${b.length}}$`,'i').test(b.draft)){b.error=`Enter ${b.length} English letters.`;return;}
    if(mode==='practice'){await run('guess');return;}
    if(b.marks.some(mark=>mark===null)){b.error='Mark every tile before applying the feedback.';return;}
    const row:HistoryRow={word:b.draft.toLowerCase(),feedback:b.marks as Mark[]};
    if(b.editing===null){if(b.history.length>=b.attempts){b.error='No attempts left. Edit an earlier row or start a new puzzle.';return;}b.history.push(row);}else b.history[b.editing]=row;
    b.editing=null;b.draft='';b.marks=Array(b.length).fill(null);b.finished=b.history.some(row=>row.feedback.every(mark=>mark==='green'));await run('analyze');
  }
  function useWord(word:string){changeDraft(word);if(mode==='instant')historyOpen=true;requestAnimationFrame(()=>guessInput?.focus());}
  async function page(offset:number){try{const target=mode, revision=b.revision;const snapshot=await bridge.wordlePage(target,offset);if(boards[target].revision===revision)accept(snapshot,target);}catch(e){b.error=message(e);}}
  async function choose(which:'answers_path'|'guesses_path'){try{const path=await bridge.choose('dictionary');if(path){prefs[which]=path;await savePrefs();}}catch(e){b.error=message(e);}}
  function keydown(event:KeyboardEvent){if(!visible || event.ctrlKey || event.altKey || event.metaKey || document.querySelector('dialog[open]'))return;const target=event.target as HTMLElement;if(target.closest('input,textarea,select'))return;if(event.key==='Enter'&&target.closest('button,summary'))return;if(event.key==='Backspace'||event.key==='Enter'||/^[a-z]$/i.test(event.key)){event.preventDefault();typeKey(event.key);}}
  onMount(()=>{document.addEventListener('keydown',keydown);return()=>{disposed=true;clearTimeout(poll);document.removeEventListener('keydown',keydown);};});
  let opened=false;
  $effect(()=>{if(visible&&!opened){opened=true;untrack(()=>{showCandidates=mode==='instant';if(mode!=='practice')void run('analyze');});}});
</script>

<section class="wordle-shell" aria-label="Wordle assistant">
  <div class="wordle-heading"><div><h2>Wordle</h2><p class="muted">{mode==='play'?'Enter feedback from your game.':mode==='instant'?'Find words from the clues you know.':'Play a local puzzle. Ask for help when you want it.'}</p></div><nav class="actions" aria-label="Wordle modes">{#each ['play','instant','practice'] as value}<button aria-pressed={mode===value} onclick={()=>switchMode(value as Mode)}>{names[value as Mode]}</button>{/each}</nav></div>
  <div class="wordle-config" aria-label="Puzzle setup">
    <label>Letters<select disabled={active} value={b.length} onchange={event=>{b.length=Number(event.currentTarget.value);void reset();}}>{#each [4,5,6,7,8] as length}<option value={length}>{length}</option>{/each}</select></label>
    <label>Attempts<select disabled={active} value={b.attempts} onchange={event=>{b.attempts=Number(event.currentTarget.value);void reset();}}>{#each Array.from({length:12},(_,i)=>i+1) as attempts}<option value={attempts}>{attempts}</option>{/each}</select></label>
    <label class="profile-select">Vocabulary<select disabled={active} value={b.profile} onchange={event=>{b.profile=event.currentTarget.value as Profile;void reset();}}><option value="broad">Broad English</option><option value="common">Common English · frequency-derived</option><option value="custom">Custom lists</option></select></label>
    <button class:primary={mode==='practice'&&!b.started} disabled={active} onclick={reset}>{mode==='practice'?'New game':'New puzzle'}</button>
    {#if active}<button onclick={()=>bridge.wordleCancel(pendingId)}>Cancel</button>{/if}
  </div>
  <p class="hint">Changing length, attempts, or vocabulary starts a new puzzle.</p>
  {#if b.profile==='custom'}<div class="wordle-custom"><label>Answer list<div class="path-field"><input readonly value={prefs.answers_path}/><button disabled={active} onclick={()=>choose('answers_path')}>Choose…</button></div></label><label>Allowed guesses <span>(optional; answers are always allowed)</span><div class="path-field"><input readonly value={prefs.guesses_path}/><button disabled={active} onclick={()=>choose('guesses_path')}>Choose…</button></div></label></div>{/if}
  {#if b.error}<div class="error" role="alert">{b.error}</div>{/if}
  <div class="wordle-columns" class:instant={mode==='instant'}>
    <div class="wordle-game">
      {#if mode==='instant'}
        <form class="wordle-constraints" onsubmit={event=>{event.preventDefault();void run('analyze');}}>
          <h3>Your clues</h3>
          <div class="pair"><label>Fixed letters<input disabled={active} bind:value={b.fixed} placeholder={'.'.repeat(b.length)} maxlength={b.length} aria-label="Fixed letters; dot for unknown"/></label><label>Attempts remaining<input disabled={active} type="number" min="0" max="12" bind:value={b.remaining}/></label></div>
          <p class="hint">Use a dot for each unknown position.</p>
          <label>Forbidden letters by position<div class="position-inputs">{#each b.forbidden as _,i}<input disabled={active} bind:value={b.forbidden[i]} aria-label={`Forbidden letters at position ${i+1}`} placeholder={`${i+1}`}/>{/each}</div></label>
          <details class="wordle-count-editor"><summary>Letter counts{b.bounds.trim()?' (set)':' (optional)'}</summary><label>Minimum and maximum<textarea disabled={active} rows="2" bind:value={b.bounds} placeholder={'e 1 2\ns 0 0'}></textarea><span class="hint">One letter, minimum, optional maximum per line. “s 0 0” excludes S.</span></label></details>
          <div class="actions"><button type="submit" class="primary" disabled={active}>Find matching words</button>{#if stale}<span class="hint">Clues changed</span>{/if}</div>
        </form>
      {/if}
      <details class="wordle-history" class:compact={mode==='instant'} open={mode!=='instant'||historyOpen} ontoggle={event=>{if(mode==='instant')historyOpen=event.currentTarget.open;}}>
        <summary>Feedback history <span class="muted">{b.history.length?`${b.history.length} ${b.history.length===1?'row':'rows'}`:'Add previous guesses'}</span></summary>
      <div class="wordle-board" style={`--board-width:${b.length*46+(b.length-1)*5}px`} aria-label="Guess history">
        {#each Array.from({length:mode==='instant'?Math.min(b.history.length+1,b.attempts):b.attempts},(_,i)=>i) as rowIndex}
          {@const old=b.history[rowIndex]}
          {@const draft=b.editing===rowIndex || (b.editing===null && rowIndex===b.history.length && !b.finished)}
          <div class="wordle-row" style={`--letters:${b.length}`}>
            {#each Array.from({length:b.length},(_,i)=>i) as i}
              {@const letter=draft?b.draft[i]:old?.word[i]}
              {@const mark=draft?b.marks[i]:old?.feedback[i]}
              <button class={`wordle-tile ${mark??''}`} class:selected={draft&&i===selected&&mode!=='practice'} disabled={active || mode==='practice' || (!draft&&!old)} aria-label={`Row ${rowIndex+1}, position ${i+1}: ${letter?.toUpperCase()??'empty'}, ${mark??'unset'}`} onclick={()=>{if(draft){selected=i;if(b.draft.length===b.length)b.marks[i]=marks[(marks.indexOf(b.marks[i])+1)%marks.length];}else editRow(rowIndex);}}>{letter?.toUpperCase()??''}<span>{mark==='green'?'●':mark==='yellow'?'◆':mark==='gray'?'×':''}</span></button>
            {/each}
          </div>
        {/each}
      </div>
      {#if b.revealed}<div class="wordle-outcome" role="status">{b.history.at(-1)?.word===b.revealed?'Solved':'Answer'}: <strong>{b.revealed.toUpperCase()}</strong></div>{/if}
      <form onsubmit={event=>{event.preventDefault();void submit();}} class="wordle-entry">
        <label>{b.editing===null?'Guess':`Edit row ${b.editing+1}`}<input bind:this={guessInput} value={b.draft} oninput={event=>changeDraft(event.currentTarget.value)} maxlength={b.length} disabled={active || (b.finished&&b.editing===null) || (mode==='practice'&&!b.started)} spellcheck="false" autocomplete="off" aria-label="Guess"/></label>
        {#if mode!=='practice'}<p class="hint">{b.draft.length===b.length?`${marked} of ${b.length} tiles marked. Select a tile or set its feedback below.`:'Enter a word, then mark each tile with your game’s feedback.'}</p><div class="feedback-palette" aria-label={`Set feedback for tile ${selected+1}`}>{#each marks as mark}<button type="button" class={mark??'unset'} aria-pressed={b.marks[selected]===mark} disabled={active||b.draft.length!==b.length} onclick={()=>{b.marks[selected]=mark;selected=Math.min(selected+1,b.length-1);}}>{mark==='green'?'● Green':mark==='yellow'?'◆ Yellow':mark==='gray'?'× Gray':'Unset'}</button>{/each}</div>{/if}
        <div class="actions"><button type="submit" class="primary" disabled={active || b.draft.length!==b.length || (mode!=='practice'&&marked!==b.length) || (b.finished&&b.editing===null) || (mode==='practice'&&!b.started)}>{mode==='practice'?'Guess':b.editing===null?'Apply feedback':'Save row'}</button>{#if b.editing!==null}<button type="button" disabled={active} onclick={removeRow}>Remove row</button><button type="button" onclick={()=>{b.editing=null;changeDraft('');}}>Cancel edit</button>{/if}</div>
      </form>
      <details class="wordle-keyboard-toggle" class:compact={mode==='instant'} open={mode!=='instant'}><summary>On-screen keyboard</summary><div class="wordle-keyboard" aria-label="On-screen keyboard">{#each ['QWERTYUIOP','ASDFGHJKL','ZXCVBNM'] as row}<div>{#if row==='ZXCVBNM'}<button disabled={active} onclick={()=>typeKey('ENTER')}>Enter</button>{/if}{#each [...row] as key}<button class={keyMarks[key]??''} disabled={active} onclick={()=>typeKey(key)} aria-label={`${key}${keyMarks[key]?`, ${keyMarks[key]}`:''}`}>{key}</button>{/each}{#if row==='ZXCVBNM'}<button disabled={active} onclick={()=>typeKey('BACKSPACE')} aria-label="Backspace">⌫</button>{/if}</div>{/each}</div></details>
      </details>
    </div>
    <div class="wordle-assistant" aria-live="polite">
      <h3>{mode==='practice'?'Assistant':mode==='instant'?'Matching words':'Your next move'}</h3>
      {#if mode==='practice' && b.started && !b.finished}<div class="actions"><button disabled={active} onclick={()=>{b.assistance=true;void run('analyze');}}>Suggest a guess</button><button disabled={active} onclick={()=>run('reveal')}>Reveal answer</button></div>{/if}
      {#if b.analysis}
        {#if stale}<p class="wordle-stale" role="status">Results are from your previous clues. Select Find matching words to update.</p>{/if}
        <p class="wordle-count">{b.analysis.candidate_count.toLocaleString()} <span>possible {b.analysis.candidate_count===1?'answer':'answers'}</span></p>
        <p class="muted">{b.analysis.profile.name} · equally likely answers · {b.analysis.attempts_remaining} attempts left</p>
        {#if b.analysis.diagnostic}<div class="error">{b.analysis.diagnostic==='contradictory_clues'?'The clues conflict. Check fixed positions, counts, and feedback.':'No words match this profile. Check the feedback or start a new puzzle with a broader answer list.'}</div>{/if}
        {#each b.analysis.notices as notice}<p class="hint">{notice}</p>{/each}
        {#if mode!=='practice'||b.assistance||b.finished}
          <p class="wordle-method">{active?'Calculating suggestions…':b.analysis.optimal_for_profile?'Optimal for this profile':b.analysis.method==='no_next_guess'?'No next guess':b.analysis.method==='two_turn_search'?'Two-attempt search':'Heuristic suggestion'}</p>
          {#each recommendations as rec,i}<article class="wordle-suggestion"><p class="muted">{i===0?'Suggested guess':'Possible answer'}{recommendations.length===1?' · possible answer':''}</p><div><strong>{rec.word.toUpperCase()}</strong><button disabled={active||b.finished||stale} onclick={()=>useWord(rec.word)}>Use word</button></div><details class="wordle-explanation"><summary>Why this guess?</summary><dl><div><dt>Information gain</dt><dd>{rec.information_bits.toFixed(2)} bits</dd></div><div><dt>Expected answers left</dt><dd>{rec.expected_remaining.toFixed(1)}</dd></div><div><dt>Immediate win</dt><dd>{percent(rec.immediate_win_probability)}</dd></div>{#if rec.solve_probability!=null}<div><dt>Solve within attempts</dt><dd>{percent(rec.solve_probability)}</dd></div>{/if}</dl></details></article>{/each}
        {:else}<p class="muted">Suggestions and word lists stay hidden until you ask for help.</p>{/if}
        <button class="wordle-list-toggle" aria-expanded={showCandidates} onclick={()=>{showCandidates=!showCandidates;if(mode==='practice')b.assistance=true;}}>{showCandidates?'Hide':'Browse'} remaining words</button>
        {#if showCandidates}<div class="wordle-word-list">{#each b.analysis.candidates as word}<button disabled={active||b.finished||stale} onclick={()=>useWord(word)}>{word.toUpperCase()}</button>{/each}</div>{#if b.analysis.candidate_count>100}<div class="actions wordle-pagination"><button disabled={active||stale||b.analysis.offset===0} onclick={()=>page(Math.max(0,(b.analysis?.offset??0)-100))}>Previous</button><span class="muted">{b.analysis.offset+1}–{Math.min(b.analysis.offset+100,b.analysis.candidate_count)}</span><button disabled={active||stale||b.analysis.offset+100>=b.analysis.candidate_count} onclick={()=>page((b.analysis?.offset??0)+100)}>Next</button></div>{/if}{/if}
        <details class="wordle-provenance"><summary>Vocabulary and search details</summary><p class="hint path">Fingerprint: {b.analysis.profile.fingerprint}</p><p class="hint">{b.analysis.profile.allowed_guesses.toLocaleString()} allowed guesses · {b.analysis.profile.eligible_answers.toLocaleString()} eligible answers across lengths<br/>Frequency {b.analysis.profile.frequency_available?'used for tie-breaking':'unavailable'} · {b.analysis.evaluated_guesses} guesses evaluated<br/>{b.analysis.guess_space_complete?'Full allowed guess space covered':'Guess space not fully evaluated'} · {b.analysis.expanded_states} search states<br/>Filtering {b.analysis.filtering_ms.toFixed(0)} ms · ranking {b.analysis.recommendation_ms.toFixed(0)} ms</p></details>
      {:else}<p class="muted">{mode==='practice'?'Choose a profile and start a new game.':'Start a new puzzle or enter the feedback you already have.'}</p>{/if}
    </div>
  </div>
</section>
