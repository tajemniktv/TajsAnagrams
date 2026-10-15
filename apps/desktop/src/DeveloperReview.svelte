<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge, message, type Job, type ReviewState, type ReviewJudgment, type ReviewPreference } from './bridge';
  import JudgmentCard from './JudgmentCard.svelte';
  import QuickReview from './QuickReview.svelte';
  let {job}: {job:Job|null}=$props();
  let evidence=$state<ReviewState>({schema_version:1,captures:[],judgments:[],preferences:[]});
  let selected=$state(''); let observation=$state(''); let busy=$state(false); let error=$state(''); let notice=$state(''); let reveal=$state(false); let contextPath=$state<string|null>(null);
  let left=$state(''); let right=$state(''); let pair=$state<[string,string]|null>(null);
  let diagnosticsOpen=$state(false);
  let page=$state(0); let savedCount=$state(0); let calibrationAnswer=$state('');
  const calibration=[{phrase:'Several Migrants Tango',reference:'useful',why:'Previously judged useful by you; a straightforward sentence reading.'},{phrase:'the and of',reference:'not_useful',why:'Curator language-only example: disconnected function words without a coherent standalone reading.'}];
  let check=$derived(calibration[Math.floor(savedCount/5)%calibration.length]);
  let cases=$derived([...new Map(evidence.captures.map(c=>[c.case_id,c])).values()]);
  let captures=$derived(evidence.captures.filter(c=>c.case_id===selected));
  let run=$derived(captures.find(c=>c.id===observation) ?? captures[0]);
  let judgments=$derived(new Map(evidence.judgments.map(j=>[j.id,j])));
  let unique=$derived([...new Map(captures.flatMap(c=>c.rows.filter(r=>r.judgeable)).map(r=>[r.id,r])).values()]);
  let judged=$derived(unique.filter(r=>['useful','not_useful'].includes(judgments.get(r.id)?.verdict ?? '')).length);
  let unsure=$derived(unique.filter(r=>judgments.get(r.id)?.verdict==='unsure').length);
  let useful=$derived(unique.filter(r=>judgments.get(r.id)?.verdict==='useful').length);
  let options=$derived([...new Map(captures.filter(c=>c.context_id===run?.context_id).flatMap(c=>c.rows.filter(r=>r.judgeable)).map(r=>[r.id,r])).values()]);
  let runUseful=$derived((run?.rows ?? []).filter(r=>r.judgeable && judgments.get(r.id)?.verdict==='useful').length);
  let runUnknown=$derived((run?.rows ?? []).filter(r=>r.judgeable && !['useful','not_useful'].includes(judgments.get(r.id)?.verdict ?? '')).length);
  async function load(){evidence=await bridge.reviewState(); if(!evidence.captures.some(c=>c.case_id===selected))selected=evidence.captures[0]?.case_id ?? '';if(!evidence.captures.some(c=>c.case_id===selected && c.id===observation))observation=evidence.captures.find(c=>c.case_id===selected)?.id ?? '';}
  async function action(f:()=>Promise<void>){busy=true;error='';notice='';try{await f();}catch(e){error=message(e);}finally{busy=false;}}
  async function save(j:ReviewJudgment){await bridge.reviewJudge(j);evidence.judgments=[...evidence.judgments.filter(x=>x.id!==j.id),j];savedCount++;calibrationAnswer='';}
  function changeCase(){observation=evidence.captures.find(c=>c.case_id===selected)?.id ?? '';page=0;pair=null;left='';right='';}
  function changeObservation(){page=0;pair=null;left='';right='';}
  function randomizePair(){if(left && right && left!==right)pair=crypto.getRandomValues(new Uint8Array(1))[0]%2 ? [right,left] : [left,right];}
  async function prefer(choice:ReviewPreference['choice']){if(!pair || !run)return;const p={capture_id:run.id,left:pair[0],right:pair[1],choice};await bridge.reviewPrefer(p);await load();notice='Pairwise preference saved separately; individual verdicts unchanged.';pair=null;}
  onMount(()=>{action(load);});
</script>
<section aria-labelledby="review-heading">
  <h2 id="review-heading">Developer review</h2>
  <p class="muted">Personal, local evidence. Captures keep original requests, displayed ranks and corpus identities. No uploads or automatic training.</p>
  <div class="actions toolbar"><button disabled={busy || !job?.id || job.active} onclick={()=>action(async()=>{await bridge.reviewCapture(job!.id);await load();notice='Current solve preserved locally.';})}>Add current solve</button><button disabled={busy} onclick={()=>action(async()=>{const p=await bridge.choose('review');if(p){const n=await bridge.reviewImport(p,null);await load();notice=`Imported ${n} observations. Matching context is detected automatically when available.`;}})}>Import benchmark</button><button disabled={busy} onclick={()=>action(async()=>{const p=await bridge.reviewExport();if(p)notice=`Exported ${p}`;})}>Export reviews</button></div>
  {#if notice}<p role="status" class="hint path">{notice}</p>{/if}{#if error}<p role="alert" class="error">{error}</p>{/if}
  <QuickReview {evidence} bind:selected onsave={save} active={!diagnosticsOpen}/>
  <details class="diagnostic-panel" bind:open={diagnosticsOpen}><summary>Optional diagnostics, calibration and comparisons</summary>
  {#if diagnosticsOpen}
  <button disabled={busy} onclick={()=>action(load)}>Refresh</button>
  <details><summary>Import existing native benchmark captures</summary><p class="hint">Choose solve JSONL, optionally its workload JSON for authentic task context/source families. No benchmark answers become judgments. Import limits: 32 MiB, 1,000 attempts. Review exports are backups, not solver JSONL.</p><div class="actions toolbar"><button disabled={busy} onclick={()=>action(async()=>{contextPath=await bridge.choose('review-workloads');})}>Choose workload context</button><button disabled={busy} onclick={()=>contextPath=null}>Clear context</button><button disabled={busy} onclick={()=>action(async()=>{const p=await bridge.choose('review');if(p){const n=await bridge.reviewImport(p,contextPath);await load();notice=`Imported ${n} new observations.`;}})}>Import JSONL</button></div><p class="hint path">{contextPath ?? 'No context selected: source-family independence and source-task fit remain unknown.'}</p></details>
  <details><summary>Judgment rubric</summary><p class="hint">Useful: straightforward coherent sentence, title, noun phrase or recognizable name; need not be outstanding. Not useful: disconnected words/broken reading requiring changes or elaborate invented context. Unsure: unfamiliar or borderline; never counted as negative. Judge actual words/order, allowing ordinary capitalization. Exact letters are checked separately. Task fit and pairwise ranking are additional judgments.</p></details>
  <div class="calibration"><strong>Calibration · language only, excluded from benchmark metrics</strong><p>{check.phrase}</p><div class="actions">{#each ['useful','unsure','not_useful'] as answer}<button aria-pressed={calibrationAnswer===answer} onclick={()=>calibrationAnswer=answer}>{answer.replaceAll('_',' ')}</button>{/each}</div>{#if calibrationAnswer}<p class="hint">Reference ({check.reference==='useful' ? 'prior personal judgment' : 'curator example'}): {check.reference.replaceAll('_',' ')}. {check.why} Disagreement can reveal a rubric issue; it is not silently corrected.</p>{/if}<p class="hint">Examples alternate after five saved output edits. These answers do not label any captured row.</p></div>
  {#if notice}<p role="status" class="hint path">{notice}</p>{/if}{#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if !cases.length}<p class="empty">No review captures yet. Finish a ranked solve and add it, or import native benchmark JSONL.</p>{:else}
  <label>Original letter bag / puzzle<select bind:value={selected} onchange={changeCase}>{#each cases as c}<option value={c.case_id}>{c.input} · {c.case_id.slice(0,32)}</option>{/each}</select></label>
  <p class="hint">{cases.length} puzzle/context groups, {evidence.captures.length} observations. Repeated trials, phrases and variants are not independent puzzles; comparisons must cluster by source family. Unknown family is not inferred.</p>
  <p class="coverage">{unique.length} distinct rendered/context outputs · {judged} definite judgments ({unique.length ? (100*judged/unique.length).toFixed(0) : 0}% coverage) · {unsure} unsure · {unique.length-judged-unsure} unreviewed · {useful} known useful. Unknown is not negative.</p>
  <label>Captured observation<select bind:value={observation} onchange={changeObservation}>{#each captures as c,i}<option value={c.id}>Observation {i+1} · {c.trial !== undefined ? `trial ${c.trial}` : 'desktop'} · {c.cache_state ?? 'native solve'} · {c.rows.length} rows</option>{/each}</select></label>
  {#if run}
  <h3>{run.input}</h3><p class="hint">Source family: {run.source_family ?? 'unknown — independence not established'} · {run.error ? `Failed: ${run.error}` : `${run.result?.generation_stop ?? 'unknown'} generation stop`}. Rank resets within each word-count bucket; no global score sorting.</p>
  <p class="coverage">This observation delivers {runUseful} known-useful rows and {runUnknown} unresolved rows: {runUseful ? 'at least one useful answer' : runUnknown ? 'useful-answer success unknown' : 'no known useful answer in this delivered list'}. This is personal standalone usefulness, not source-task success.</p>
  {#if run.source_context?.original_context}<p class="hint">Original task: {run.source_context.original_context}</p>{/if}
  <details><summary>Original request, constraints and provenance (may reveal treatment)</summary><pre>{JSON.stringify({request:run.request,source_context:run.source_context,mapping:run.task_mapping,status:run.result?.status ?? run.status,origin:run.origin},null,2)}</pre></details>
  <label class="check"><input type="checkbox" bind:checked={reveal}/>Reveal model scores and treatment identity (not human quality)</label>{#if reveal}<p class="hint">Treatment: {run.dictionary_variant ?? 'desktop configured inputs'}</p>{/if}
  <div class="actions toolbar"><button disabled={page===0} onclick={()=>page--}>Previous</button><span>Rows {run.rows.length ? page*20+1 : 0}–{Math.min((page+1)*20,run.rows.length)} / {run.rows.length}</span><button disabled={(page+1)*20>=run.rows.length} onclick={()=>page++}>Next</button></div>
  {#key run.id}{#each run.rows.slice(page*20,(page+1)*20) as row (`${row.id}-${row.word_count}-${row.rank}`)}{#if reveal}<p class="hint">Model score: {row.score?.toFixed(2) ?? 'unavailable'}</p>{/if}<JudgmentCard {row} value={judgments.get(row.id)} onsave={save}/>{/each}{/key}
  {#if !run.rows.length}<p class="hint">No published phrases. Failure/empty delivery is retained; there is no phrase to rate.</p>{/if}
  <details class="pairwise"><summary>Pairwise ranking preference · additional to individual judgments</summary><p class="hint">Select two actual outputs from this puzzle/context across its observations. Presentation is randomized. Both may be good or both bad; preferences do not overwrite Useful/Unsure/Not useful.</p><label>First candidate<select bind:value={left}><option value="">Choose</option>{#each options as r}<option value={r.id}>{r.phrase}</option>{/each}</select></label><label>Second candidate<select bind:value={right}><option value="">Choose</option>{#each options as r}<option value={r.id}>{r.phrase}</option>{/each}</select></label><button disabled={!left || !right || left===right || busy} onclick={randomizePair}>Present randomized pair</button>{#if pair}<div class="pair"><p><strong>A:</strong> {options.find(r=>r.id===pair![0])?.phrase}</p><p><strong>B:</strong> {options.find(r=>r.id===pair![1])?.phrase}</p></div><div class="actions toolbar">{#each [['left','Prefer A'],['right','Prefer B'],['tie','Equally useful'],['both_bad','Both not useful'],['unsure','Unsure']] as [choice,label]}<button disabled={busy} onclick={()=>action(()=>prefer(choice as ReviewPreference['choice']))}>{label}</button>{/each}</div>{/if}<p class="hint">{evidence.preferences.filter(p=>p.capture_id===run.id).length} saved pairwise judgments saved from this observation; identities retain the common context.</p></details>
  {/if}{/if}
  <p class="hint path">Storage: {evidence.storage ?? 'Created on first capture'} · Save notes before changing observations or leaving this tab. Clear result cache and disabling developer mode do not delete reviews. Exports contain puzzle text, personal notes and configured local paths: review before sharing.</p>
  {/if}
  </details>
</section>
<style>
  section>.muted,section>.hint,.coverage{margin:12px 0}.toolbar{flex-wrap:wrap;margin:14px 0}details{border-top:1px solid var(--border);padding:12px 0}summary{cursor:pointer}.calibration{padding:15px;border:1px dashed var(--border);border-radius:8px;margin:16px 0}.calibration p{margin:10px 0}.calibration strong{font-size:12px}.calibration .actions{flex-wrap:wrap}.coverage{padding:12px;background:var(--panel);border-radius:8px;line-height:1.5;font-size:13px}pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:350px;overflow:auto;font-size:12px}.pairwise .pair{margin:15px 0}section label{margin-top:14px}button[aria-pressed=true]{color:var(--accent);border-color:var(--accent)}
</style>
