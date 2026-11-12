import type { Mark } from '../../../contracts/generated/WordleSnapshot';

export interface PuzzleDraft {
  length:number; attempts:number; profile:'broad'|'common'|'custom';
  entries:({word:string;feedback:(Mark|null)[];ready:boolean}|null)[];
  activeRow:number; draft:string; marks:(Mark|null)[]; cursor:number|null; ready:boolean;
  fixed:string; forbidden:string[]; bounds:string; remaining:number;
  fingerprint:string|null;
}
const key='tajs-wordle-session-v1';
const integer=(n:unknown,min:number,max:number):n is number=>Number.isInteger(n)&&Number(n)>=min&&Number(n)<=max;
const text=(s:unknown,max:number):s is string=>typeof s==='string'&&s.length<=max;
const feedback=(v:unknown,n:number):v is (Mark|null)[]=>Array.isArray(v)&&v.length===n&&v.every(m=>m===null||['gray','yellow','green'].includes(m));
function valid(v:unknown):v is PuzzleDraft {
  if(!v||typeof v!=='object')return false;
  const p=v as PuzzleDraft;
  return integer(p.length,4,8)&&integer(p.attempts,1,12)&&['broad','common','custom'].includes(p.profile)
    &&integer(p.activeRow,0,p.attempts-1)&&text(p.draft,p.length)&&/^[a-z ]*$/i.test(p.draft)
    &&feedback(p.marks,p.length)&&(p.cursor===null||integer(p.cursor,0,p.length-1))&&typeof p.ready==='boolean'
    &&text(p.fixed,p.length)&&p.fixed.length===p.length&&/^[a-z.]+$/i.test(p.fixed)
    &&Array.isArray(p.forbidden)&&p.forbidden.length===p.length&&p.forbidden.every(s=>text(s,26)&&/^[a-z]*$/i.test(s))
    &&text(p.bounds,1024)&&integer(p.remaining,0,12)&&(p.fingerprint===null||text(p.fingerprint,512))
    &&Array.isArray(p.entries)&&p.entries.length===p.attempts&&p.entries.every(r=>r===null||(text(r.word,p.length)&&/^[a-z ]*$/i.test(r.word)&&feedback(r.feedback,p.length)&&typeof r.ready==='boolean'));
}
export function puzzleDraft(board:PuzzleDraft):PuzzleDraft {
  const {length,attempts,profile,entries,activeRow,draft,marks,cursor,ready,fixed,forbidden,bounds,remaining,fingerprint}=board;
  return {length,attempts,profile,entries:entries.map(r=>r?{word:r.word,feedback:[...r.feedback],ready:r.ready}:null),activeRow,draft,marks:[...marks],cursor,ready,fixed,forbidden:[...forbidden],bounds,remaining,fingerprint};
}
export function readSession(storage:Storage|undefined=typeof window==='undefined'?undefined:window.localStorage):{play:PuzzleDraft;instant:PuzzleDraft}|null {
  const raw=storage?.getItem(key);
  if(!raw)return null;
  if(raw.length>32768)throw new Error('Saved Wordle puzzle is too large. Start a new puzzle.');
  const saved=JSON.parse(raw);
  if(saved.schema!==1||!valid(saved.play)||!valid(saved.instant))throw new Error('Saved Wordle puzzle is incompatible. Start a new puzzle.');
  return {play:puzzleDraft(saved.play),instant:puzzleDraft(saved.instant)};
}
export function saveSession(play:PuzzleDraft,instant:PuzzleDraft,storage:Storage|undefined=typeof window==='undefined'?undefined:window.localStorage) {
  storage?.setItem(key,JSON.stringify({schema:1,play:puzzleDraft(play),instant:puzzleDraft(instant)}));
}
