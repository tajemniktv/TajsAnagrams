<script lang="ts">
  import { onMount } from 'svelte';
  import { Settings as SettingsIcon, Sun, Moon, Square } from '@lucide/svelte';
  import { bridge, message, type Bootstrap, type Job, type Settings, type SolveResult, type ActiveOperation } from './bridge';
  import Advanced from './Advanced.svelte';
  import SettingsDialog from './SettingsDialog.svelte';
  import Results from './Results.svelte';
  import DeveloperReview from './DeveloperReview.svelte';
  import Wordle from './Wordle.svelte';
  let appTab = $state<'anagrams' | 'wordle'>('anagrams');
  let wordleActive = $state(false);
  let operation = $state<ActiveOperation | null>(null);
  let operationPoll: ReturnType<typeof setTimeout>;
  let boot = $state<Bootstrap | null>(null); let settings = $state<Settings | null>(null);
  let job = $state<Job | null>(null); let result = $state<SolveResult | null>(null);
  let error = $state(''); let toast = $state(''); let showSettings = $state(false); let busy = $state(false); let cancelling = $state(false);
  let poll: ReturnType<typeof setTimeout>; let disposed = false;
  let required = $state(''); let hints = $state(''); let excluded = $state('');
  let extended = $state(false); let rebuild = $state(false);
  let generationOnly = $state(false);
  let workspaceTab = $state<'results' | 'review'>('results');
  let active = $derived(busy || !!job?.active || wordleActive || !!operation);
  async function refreshOperation() {
    try { const next=await bridge.operationStatus(); if(!disposed)operation=next; }
    catch(e) { if(!disposed)error=message(e); }
    if(!disposed)operationPoll=setTimeout(refreshOperation,250);
  }
  async function cancelOperation() { if(operation)try { await bridge.cancelOperation(operation.id); await refreshOperationOnce(); } catch(e) { error=message(e); } }
  async function refreshOperationOnce() { const next=await bridge.operationStatus(); if(!disposed)operation=next; }
  let letters = $derived(settings?.request.generation.text.normalize('NFKD').replace(/[^a-z]/gi, '').length ?? 0);
  const words = (text: string) => text.split(/[\s,]+/).filter(Boolean);
  function requireWord(word:string){if(active)return;required=[...new Set([...words(required),word])].join('\n');excluded=words(excluded).filter(w=>w!==word).join('\n');toast=`Required ${word}; solve again to apply`;}
  function excludeWord(word:string){if(active)return;excluded=[...new Set([...words(excluded),word])].join('\n');required=words(required).filter(w=>w!==word).join('\n');toast=`Excluded ${word}; solve again to apply`;}
  function applyTheme(theme: Settings['theme']) { document.documentElement.dataset.theme = theme; }
  async function load() {
    try { boot = await bridge.bootstrap(); settings = boot.settings; settings.request.workers ??= 1; settings.request.refine ??= false; applyTheme(settings.theme);
      required = settings.request.generation.required?.join('\n') ?? ''; hints = settings.request.generation.hints?.join('\n') ?? ''; excluded = settings.request.generation.excluded?.join('\n') ?? '';
      error = boot.notice ?? boot.corpus_error ?? ''; await refresh();
    } catch (e) { error = `Desktop connection unavailable: ${message(e)}. Launch the installed TajsAnagrams app, not a browser preview.`; }
  }
  async function refresh() {
    if (disposed) return;
    try { job = await bridge.status(); if (!job.active && job.id) { cancelling = false; result = await bridge.result(job.id); if (job.error && job.status?.state !== 'cancelled') error = job.error; }
      if (job.active) poll = setTimeout(refresh, 250);
    } catch (e) { error = message(e); }
  }
  async function start(event: SubmitEvent) {
    event.preventDefault(); if (!settings || active) return;
    settings.request.generation.required = words(required); settings.request.generation.hints = words(hints); settings.request.generation.excluded = words(excluded);
    busy = true; error = ''; toast = ''; cancelling = false; result = null;
    try { await bridge.start($state.snapshot(settings.request), rebuild, extended, generationOnly && settings.developer_mode); await refresh(); } catch (e) { error = message(e); } finally { busy = false; }
  }
  async function cancel() { try { if (job?.active) { await bridge.cancel(job.id); cancelling = true; } } catch (e) { error = message(e); } }
  async function save(next: Settings) { const saved = $state.snapshot(next); await bridge.save(saved); settings = saved; applyTheme(saved.theme); boot = await bridge.bootstrap(); error = boot.corpus_error ?? ''; toast = 'Settings saved'; }
  async function toggleTheme() { if (!settings) return; const next = structuredClone($state.snapshot(settings)); next.theme = next.theme === 'light' ? 'dark' : 'light'; try { await save(next); } catch (e) { error = message(e); } }
  async function copy() { try { if (job) { await bridge.copy(job.id); toast = 'Phrases copied to clipboard'; } } catch (e) { error = message(e); } }
  async function exportRows(format: 'txt' | 'json') { try { if (job) { const path = await bridge.export(job.id, format); if (path) toast = `Exported to ${path}`; } } catch (e) { error = message(e); } }
  onMount(() => { load(); void refreshOperation(); return () => { disposed = true; clearTimeout(poll); clearTimeout(operationPoll); }; });
</script>
<div class="app-shell">
  <header><h1 style="display:flex;align-items:center;gap:10px"><img src="/logo.png" alt="" width="34" height="34"/>TajsAnagrams</h1><div class="header-actions"><span class="local"><i></i>Local engine</span><button class="icon" disabled={active || !settings} aria-label="Toggle appearance" onclick={toggleTheme}>{#if settings?.theme === 'light'}<Moon size={19}/>{:else}<Sun size={19}/>{/if}</button><button disabled={active || !settings} onclick={() => showSettings = true}><SettingsIcon size={17}/>Settings</button></div></header>
  <nav class="app-tabs" aria-label="Puzzle tools"><button aria-pressed={appTab==='anagrams'} onclick={()=>appTab='anagrams'}>Anagrams</button><button aria-pressed={appTab==='wordle'} onclick={()=>appTab='wordle'}>Wordle</button></nav>
  {#if operation}<div class="operation-status" role="status"><span>{operation.kind==='wordle'?'Wordle analysis':operation.kind==='anagrams'?'Anagram search':'Data or training operation'}{operation.cancelling?' · stopping…':' is running'}</span><button disabled={operation.cancelling} onclick={cancelOperation}>Cancel operation</button></div>{/if}
  <main hidden={appTab!=='anagrams'}>
    <aside>
      <h2>Find the words</h2>
      {#if settings}
      <form onsubmit={start}>
        <fieldset disabled={active}>
          <label>Letters or phrase<textarea class="target" required maxlength={extended || settings.runtime.custom_limits ? undefined : 4096} bind:value={settings.request.generation.text} placeholder="I AM TESTING ANAGRAMS" spellcheck="false"></textarea></label>
          <p class="hint counter">{letters} letters{extended ? ' · extended local search' : settings.runtime.custom_limits ? ' · custom execution limits' : ' · maximum 40'}</p>
          <label>Required words <span>(one per line)</span><textarea rows="2" bind:value={required} placeholder="e.g. testing"></textarea></label>
          <label>Hint words <span>(one per line)</span><textarea rows="2" bind:value={hints} placeholder="At least one should appear"></textarea></label>
          <label>Exclude words <span>(one per line)</span><textarea rows="2" bind:value={excluded} placeholder="Words to leave out"></textarea></label>
        </fieldset>
        <Advanced bind:request={settings.request} bind:extended bind:rebuild {generationOnly} custom={settings.runtime.custom_limits} disabled={active}/>
        {#if settings.developer_mode}<label class="check"><input disabled={active} type="checkbox" bind:checked={generationOnly}/>Generate word bags only (skip ranking)</label>{/if}
        {#if job?.active}<button class="primary solve" type="button" disabled={busy || cancelling} onclick={cancel}><Square size={16}/>Cancel solve</button>{:else}<button class="primary solve" type="submit" disabled={active || !letters || (!extended && !settings.runtime.custom_limits && letters > 40)}>{active?'Worker in use':generationOnly && settings.developer_mode ? 'Generate word bags' : 'Solve anagram'}</button>{/if}
      </form>
      {:else}<p class="muted">Connecting to the local engine…</p>{/if}
    </aside>
    <div class="workspace">
      {#if error}<div class="error" role="alert">{error}</div>{/if}
      {#if settings?.developer_mode}<div class="actions workspace-tabs" aria-label="Workspace views"><button aria-pressed={workspaceTab === 'results'} onclick={() => workspaceTab = 'results'}>Results</button><button aria-pressed={workspaceTab === 'review'} onclick={() => workspaceTab = 'review'}>Developer review</button></div>{/if}
      {#if settings?.developer_mode && workspaceTab === 'review'}<DeveloperReview {job}/>{:else}<Results {result} {job} oncopy={copy} onexport={exportRows} onrequire={requireWord} onexclude={excludeWord}/>{/if}
    </div>
  </main>
  <div class="wordle-view" hidden={appTab!=='wordle'}>{#if settings}<Wordle {settings} onpreferences={prefs=>{if(settings)settings.wordle=prefs;}} blocked={busy || !!job?.active || (!!operation && operation.kind!=='wordle')} frequencyAvailable={!!boot?.wordle_frequencies_available} visible={appTab==='wordle'} bind:active={wordleActive}/>{/if}</div>
  <footer role="status"><span>{wordleActive ? 'Analyzing Wordle' : active && cancelling ? 'Cancellation requested. Waiting for the worker to stop…' : toast || (active ? 'Solving locally' : job?.status?.state.replaceAll('_', ' ') ?? (job?.error ? 'Solve failed' : 'Ready'))}</span><span>Runs entirely on this PC{appTab==='wordle'?'':extended?' · extended search':settings?.runtime.custom_limits?' · custom execution limits':' · 120 s job limit'}</span></footer>
</div>
{#if showSettings && settings && boot}<SettingsDialog {settings} directory={boot.data_directory} onsave={save} onclose={() => showSettings = false}/>{/if}
