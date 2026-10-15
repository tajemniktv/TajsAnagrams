<script lang="ts">
  import type {ReviewState, ReviewJudgment} from './bridge';
  import JudgmentCard from './JudgmentCard.svelte';
  let {evidence,selected=$bindable(''),onsave,active=true}: {evidence:ReviewState;selected?:string;onsave:(j:ReviewJudgment)=>Promise<void>;active?:boolean}=$props();
  let previous=$state('');let focus=$state('');let saved=$state(0);
  let cases=$derived([...new Map(evidence.captures.map(c=>[c.case_id,c])).values()]);
  let labels=$derived(new Map(evidence.judgments.map(j=>[j.id,j])));
  let captures=$derived(evidence.captures.filter(c=>c.case_id===selected));
  let rows=$derived([...new Map(captures.flatMap(c=>c.rows.filter(r=>r.judgeable)).map(r=>[r.id,r])).values()]);
  let pending=$derived(rows.filter(r=>!labels.get(r.id)?.verdict));
  let current=$derived(rows.find(r=>r.id===focus) ?? pending[0]);
  let input=$derived(captures[0]?.input ?? '');
  let maybe=$derived(rows.filter(r=>labels.get(r.id)?.verdict==='unsure').length);
  function change(){focus='';previous='';}
  function rated(id:string){previous=id;focus='';saved++;}
  function nextPuzzle(){const next=cases.find(c=>c.case_id!==selected && evidence.captures.filter(r=>r.case_id===c.case_id).some(r=>r.rows.some(row=>row.judgeable && !labels.get(row.id)?.verdict)));if(next){selected=next.case_id;change();}}
</script>
<div class="quick-review">
  <p class="muted">One phrase, one click. Yes = useful on its own; Maybe = uncertain; No = not useful. Sentences, titles, noun phrases and recognizable names count. No explanations required.</p>
  {#if cases.length}
    <label>Puzzle<select aria-label="Puzzle" bind:value={selected} onchange={change}>{#each cases as c}<option value={c.case_id}>{c.input}</option>{/each}</select></label>
    <h3>{input}</h3>
    <p class="hint">{rows.length-pending.length} / {rows.length} distinct outputs reviewed · {maybe} Maybe (still uncertain) · {saved} saved this session. Repeated trials/treatments are pooled, not new puzzles.</p>
    {#if !rows.length}<p class="complete">No published phrases to review for this puzzle. Empty or failed observations remain available under diagnostics.</p><button onclick={nextPuzzle}>Next unreviewed puzzle</button>
    {:else if current}
      <p class="hint">The rank below is a captured position within its word-count group, not a global rank. This queue is not a ranked result list.</p>
      {#key current.id}<JudgmentCard row={current} value={labels.get(current.id)} {onsave} quick shortcutEnabled={active} onrated={rated}/>{/key}
      <div class="actions"><button disabled={!previous} onclick={()=>focus=previous}>Previous / revise</button>{#if focus}<button onclick={()=>focus=''}>Continue new outputs</button>{/if}</div>
      <p class="hint">Keys 1 / 2 / 3 also work, except while typing a note. Votes save and advance automatically. Stop whenever; your progress is kept.</p>
    {:else}<p class="complete">You have reviewed every output in this puzzle. Maybe ratings remain unresolved; they are not counted as No.</p><button onclick={nextPuzzle}>Next unreviewed puzzle</button>{/if}
  {:else}<p class="complete">Add a completed solve or import a native capture to begin. Existing reviews will be preserved.</p>{/if}
</div>
<style>.quick-review{max-width:850px}.quick-review>.muted{margin:12px 0 20px}.quick-review .hint{margin:12px 0}.complete{margin:28px 0;line-height:1.6}label{margin-bottom:18px}h3{overflow-wrap:anywhere}</style>
