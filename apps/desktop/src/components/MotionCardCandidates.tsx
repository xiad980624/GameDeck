import {useEffect,useRef,useState} from 'react';
import {invoke,convertFileSrc} from '@tauri-apps/api/core';
import {MotionImage} from './MotionImage';
import {useDeck} from '../lib/store';
type Choice={id:string;path:string;width:number;height:number;duration_ms:number;approved:boolean};
export function MotionCardCandidates({gameId,kind,onApplied}:{gameId:string;kind:'poster'|'landscape';onApplied:()=>Promise<void>}){
 const [items,setItems]=useState<Choice[]>([]);const [selected,setSelected]=useState<Choice|null>(null);const [busy,setBusy]=useState(false);const alive=useRef(true);const notify=useDeck(s=>s.notify);
 useEffect(()=>{alive.current=true;void invoke<Choice[]>('card_motion_candidates',{gameId,kind,refresh:false}).then(value=>{if(alive.current)setItems(value);}).catch(e=>notify(String(e)));return()=>{alive.current=false;};},[gameId,kind]);
 const search=async()=>{setBusy(true);try{const value=await invoke<Choice[]>('card_motion_candidates',{gameId,kind,refresh:true});if(alive.current){setItems(value);setSelected(null);if(!value.length)notify('没有找到符合尺寸与时长要求的动态卡片');}}catch(e){notify(String(e));}finally{if(alive.current)setBusy(false);}};
 const apply=async()=>{if(!selected)return;setBusy(true);try{await invoke('choose_motion_card',{gameId,kind,candidateId:selected.id});await onApplied();notify('动态卡片已应用并锁定');}catch(e){notify(String(e));}finally{if(alive.current)setBusy(false);}};
 return <div className={`background-candidates motion-card-${kind}`}><div className="setting-row"><h3>动态卡片候选</h3><button className="btn btn-outline" disabled={busy} onClick={()=>void search()}>{busy?'处理中…':'搜索动态卡片'}</button></div><p className="setting-note">选择游戏名称或 Logo 清晰、持续可辨的卡面。动态图开关打开后播放。</p>
 <div className="background-candidate-grid">{items.map(item=><button key={item.id} className={selected?.id===item.id?'candidate-selected':''} disabled={busy} onClick={()=>setSelected(item)}><MotionImage src={convertFileSrc(item.path)} play={selected?.id===item.id} alt="动态卡片候选"/><span>{item.width} × {item.height} · {(item.duration_ms/1000).toFixed(1)} 秒</span><small>{item.approved?'已确认游戏标识':'待确认游戏标识'}</small></button>)}</div>
 {selected&&<button className="btn btn-primary" disabled={busy} onClick={()=>void apply()}>使用并锁定此动图</button>}
 </div>;
}
