import {useEffect,useRef,useState} from 'react';
import {invoke,convertFileSrc} from '@tauri-apps/api/core';
import {MotionImage} from './MotionImage';
import {useDeck} from '../lib/store';
type Candidate={id:string;path:string;width:number;height:number;animated:boolean;duration_ms:number;title:string;source:string;reason:string};
export function BackgroundCandidates({gameId,onApplied}:{gameId:string;onApplied:()=>Promise<void>}){
 const [items,setItems]=useState<Candidate[]>([]);const [selected,setSelected]=useState<Candidate|null>(null);const [busy,setBusy]=useState(false);
 const alive=useRef(true);const notify=useDeck(s=>s.notify);
 useEffect(()=>{alive.current=true;void invoke<Candidate[]>('background_candidates',{gameId,refresh:false}).then(setItems).catch(e=>notify(String(e)));return()=>{alive.current=false;};},[gameId]);
 const search=async()=>{setBusy(true);try{const next=await invoke<Candidate[]>('background_candidates',{gameId,refresh:true});if(alive.current){setItems(next);setSelected(null);if(!next.length)notify('没有找到合格的高清候选，保留现有背景');}}catch(e){if(alive.current)notify(String(e));}finally{if(alive.current)setBusy(false);}};
 const apply=async()=>{if(!selected)return;setBusy(true);try{await invoke('choose_background',{gameId,candidateId:selected.id});await onApplied();notify('背景已锁定，自动刮削不会覆盖');}catch(e){notify(String(e));}finally{if(alive.current)setBusy(false);}};
 return <div className="background-candidates"><div className="setting-row"><h3>背景候选</h3><button className="btn btn-outline" disabled={busy} onClick={()=>void search()}>{busy?'处理中…':'搜索高清背景'}</button></div>
 <p className="setting-note">优先 4K 宣传图，无 4K 时回退到 2K；标记为疑似截图的素材降低优先级。</p>
 <div className="background-candidate-grid">{items.map(item=><button key={item.id} className={selected?.id===item.id?'candidate-selected':''} disabled={busy} onClick={()=>setSelected(item)}><MotionImage src={convertFileSrc(item.path)} play={false} alt={item.title}/><span>{item.width} × {item.height} · {item.width>=3840&&item.height>=2160?'4K':'2K'}{item.animated?` · 动态 ${(item.duration_ms/1000).toFixed(1)} 秒`:''}</span><small>{item.reason} · {item.source.replace('background-','')}</small></button>)}</div>
 {selected&&<><div className="candidate-preview"><MotionImage src={convertFileSrc(selected.path)} alt={selected.title}/><div className="hero-overlay"/></div><button className="btn btn-primary" disabled={busy} onClick={()=>void apply()}>使用并锁定此背景</button></>}
 </div>;
}
