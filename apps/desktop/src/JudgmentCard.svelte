<script lang="ts">
  import { untrack } from 'svelte';
  import { ThumbsUp, ThumbsDown, CircleHelp } from '@lucide/svelte';
  import { message, type ReviewRow, type ReviewJudgment, type Verdict } from './bridge';
  let { row, value, onsave, quick = false, shortcutEnabled = true, onrated }: {row: ReviewRow; value?: ReviewJudgment; onsave: (v: ReviewJudgment) => Promise<void>; quick?: boolean; shortcutEnabled?: boolean; onrated?: (id: string) => void} = $props();
  let draft = $state<ReviewJudgment>(untrack(() => value ? structuredClone($state.snapshot(value)) : {id:row.id,verdict:null,reasons:[],note:'',task_fit:null,looked_up:false}));
  let busy = $state(false); let error = $state(''); let status = $state('');
  const reasons = [
    ['Language form','grammar','Grammar / word order'],['Language form','spelling_rendering','Spelling / punctuation'],
    ['Meaning','meaning','Meaning does not fit together'],['Meaning','contrived_context','Needs contrived context'],
    ['Meaning','incomplete_meaning','Incomplete meaning / missing relation'],['Meaning','semantic_anomaly','Semantic anomaly despite plausible grammar'],
    ['Knowledge','unfamiliar','Unfamiliar word'],['Knowledge','unverified_name','Unverified proper name'],
    ['Task','task_mismatch','Does not fit source task'],['Other','unspecified','Unspecified'],
  ];
  function reasonChanged(key: string) {
    if(draft.reasons.includes(key)) draft.reasons=key==='unspecified' ? ['unspecified'] : draft.reasons.filter(r=>r!=='unspecified');
    void save();
  }
  async function save(verdict: Verdict | null = draft.verdict, advance = false) {
    if(busy) return; busy=true; error=''; status='';
    const next=structuredClone($state.snapshot(draft)); next.verdict=verdict;
    if(verdict==='not_useful' && !next.reasons.length) next.reasons=['unspecified'];
    try {await onsave(next); draft=next; status='Saved locally'; if(advance) onrated?.(next.id);} catch(e){error=message(e);} finally{busy=false;}
  }
  function shortcut(event: KeyboardEvent) {
    if(!quick || !shortcutEnabled || busy || event.repeat || event.ctrlKey || event.metaKey || event.altKey || document.querySelector('dialog[open]') || (event.target instanceof Element && event.target.closest('input,textarea,select,[contenteditable]')))return;
    const v=({'1':'useful','2':'unsure','3':'not_useful'} as Record<string,Verdict>)[event.key];
    if(v){event.preventDefault();void save(v,true);}
  }
</script>
<svelte:window onkeydown={shortcut}/>
<article class:quick class="judgment-card">
  <div class="row-title"><span class="rank">{row.word_count} words · rank {row.rank}</span><strong>{row.phrase || '[Unordered word bag — not judgeable]'}</strong></div>
  {#if row.judgeable}
  <div class="actions votes" aria-label={`Usefulness of ${row.phrase}`}>
    <button disabled={busy} aria-pressed={draft.verdict==='useful'} aria-label={`Useful: ${row.phrase}`} onclick={() => save('useful',quick)}><ThumbsUp size={16}/>{quick ? 'Yes · 1' : 'Useful'}</button>
    <button disabled={busy} aria-pressed={draft.verdict==='unsure'} aria-label={`Unsure: ${row.phrase}`} onclick={() => save('unsure',quick)}><CircleHelp size={16}/>{quick ? 'Maybe · 2' : 'Unsure'}</button>
    <button disabled={busy} aria-pressed={draft.verdict==='not_useful'} aria-label={`Not useful: ${row.phrase}`} onclick={() => save('not_useful',quick)}><ThumbsDown size={16}/>{quick ? 'No · 3' : 'Not useful'}</button>
    {#if !quick}<button disabled={busy || !draft.verdict} onclick={() => save(null)}>Clear verdict</button>{/if}
  </div>
  <details><summary>{quick ? 'Optional: add a note or reason' : 'Reasons, short note and source-task fit'}</summary>
  {#if draft.verdict === 'not_useful' || draft.verdict === 'unsure'}
    <fieldset disabled={busy} class="reason-grid"><legend>Diagnostic reasons (multiple allowed; not automatic labels)</legend>
      {#each ['Language form','Meaning','Knowledge','Task','Other'] as group}<div><span class="hint">{group}</span>{#each reasons.filter(r=>r[0]===group) as [,key,label]}<label class="check"><input type="checkbox" value={key} bind:group={draft.reasons} onchange={() => reasonChanged(key)}/>{label}</label>{/each}</div>{/each}
    </fieldset>
  {/if}
  <label class="note-label">Short note for this output<textarea disabled={busy} maxlength="2000" rows="2" bind:value={draft.note} oninput={() => status='Unsaved note — use Save note'} placeholder="What reading works, or what fails? Notes need Save note." aria-label={`Note: ${row.phrase}`}></textarea></label>
  <div class="note-actions"><label class="check"><input disabled={busy} type="checkbox" bind:checked={draft.looked_up} onchange={() => save()}/>Looked up a word/name</label><button disabled={busy} onclick={() => save()}>Save note</button></div>
  <label>Fit to source task (separate; leave unreviewed without context)<select disabled={busy} bind:value={draft.task_fit} onchange={() => save()} aria-label={`Task fit: ${row.phrase}`}><option value={null}>Unreviewed</option><option value="useful">Fits task</option><option value="not_useful">Does not fit task</option><option value="unsure">Unsure</option></select></label>
  </details>
  {#if error}<p role="alert" class="error">{error}</p>{:else if status}<p role="status" class="hint">{status}</p>{/if}
  {/if}
</article>
<style>
  .judgment-card{border:1px solid var(--border);border-radius:9px;padding:16px;margin:12px 0}.row-title{display:flex;gap:14px;align-items:baseline;flex-wrap:wrap;margin-bottom:12px}.row-title strong{font-size:17px}.rank{color:var(--muted);font-size:12px}.votes{flex-wrap:wrap}.votes [aria-pressed=true]{border-color:var(--accent);background:var(--panel);color:var(--accent)}.reason-grid{margin-top:15px;display:flex;gap:15px;flex-wrap:wrap}.reason-grid legend{font-size:12px;color:var(--muted);margin-bottom:8px}.reason-grid .check{margin:7px 0;font-size:12px}.note-label{margin:14px 0 8px}.note-actions{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:12px}.note-actions .check{margin:0}label{font-weight:400}
</style>
