<script lang="ts">
  let {value,length,onchange}:{value:string;length:number;onchange:(value:string)=>void}=$props();
  let letter=$state('');
  let rows=$derived(value.split('\n').filter(s=>s.trim()).map(s=>s.trim().split(/\s+/)));
  function update(index:number,field:number,text:string){const next=rows.map(r=>[...r]);next[index][field]=text;onchange(next.map(r=>r.join(' ').trim()).join('\n'));}
  function add(){const ch=letter.replace(/[^a-z]/gi,'').slice(0,1).toLowerCase();if(!ch)return;onchange([...rows.map(r=>r.join(' ')),`${ch} 1`].join('\n'));letter='';}
</script>
<fieldset class="letter-counts"><legend>Letter counts</legend>
  {#each rows as row,i}<div class="count-row"><label>Letter<input aria-label={`Count ${i+1} letter`} maxlength="1" value={row[0]} oninput={e=>update(i,0,e.currentTarget.value)}/></label><label>Minimum<input aria-label={`Count ${i+1} minimum`} type="number" min="0" max={length} value={row[1]} onchange={e=>update(i,1,e.currentTarget.value)}/></label><label>Maximum<input aria-label={`Count ${i+1} maximum`} type="number" min="0" max={length} placeholder="Any" value={row[2]??''} onchange={e=>update(i,2,e.currentTarget.value)}/></label><button type="button" aria-label={`Remove count ${i+1}`} onclick={()=>onchange(rows.filter((_,index)=>index!==i).map(r=>r.join(' ')).join('\n'))}>Remove</button></div>{/each}
  <div class="actions"><input aria-label="Letter to count" maxlength="1" bind:value={letter}/><button type="button" disabled={!/^[a-z]$/i.test(letter)} onclick={add}>Add letter</button></div>
  <details><summary>Text input</summary><textarea aria-label="Letter count syntax" rows="3" {value} oninput={e=>onchange(e.currentTarget.value)} placeholder={'e 1 2\ns 0 0'}></textarea><p class="hint">Letter, minimum, optional maximum. S 0 0 excludes S.</p></details>
</fieldset>
<style>.letter-counts{margin-bottom:18px}.count-row{display:grid;grid-template-columns:1fr 1fr 1fr auto;gap:8px;align-items:end}.count-row label{min-width:0}.count-row input{min-width:0}.count-row button{margin-bottom:18px}.actions input{width:64px;margin:0}legend{font-weight:600;font-size:13px;margin-bottom:8px}</style>
