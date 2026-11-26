import { MotionImage } from './MotionImage';
import { BackgroundCandidates } from './BackgroundCandidates';
import { MotionCardCandidates } from './MotionCardCandidates';
import {useEffect,useRef,useState} from 'react';
import {invoke,convertFileSrc} from '@tauri-apps/api/core';
import {open} from '@tauri-apps/plugin-dialog';
import {backend} from '../lib/backend';
import {useDeck} from '../lib/store';
type Kind='hero'|'poster'|'landscape'|'logo';
type Choice={kind:Kind;path:string;width:number;height:number;manual:boolean};
type Draft={id:string;game_id:string;kind:Kind;path:string;width:number;height:number};
const kinds:{id:Kind;name:string}[]=[{id:'hero',name:'背景'},{id:'poster',name:'竖封面'},{id:'landscape',name:'横卡片'},{id:'logo',name:'Logo'}];
export function ManualArtwork({gameId}:{gameId:string}){
 const [kind,setKind]=useState<Kind>('hero');const [choices,setChoices]=useState<Choice[]>([]);
 const [draft,setDraft]=useState<Draft|null>(null);const [busy,setBusy]=useState(false);const [fill,setFill]=useState(false);
 const alive=useRef(true);useEffect(()=>{alive.current=true;return()=>{alive.current=false;};},[]);
 const notify=useDeck(s=>s.notify);
 const refresh=async()=>{const [next,games]=await Promise.all([invoke<Choice[]>('artwork_choices',{gameId}),backend.listGames()]);if(alive.current){setChoices(next);useDeck.getState().setGames(games);}};
 useEffect(()=>{void invoke<Choice[]>('artwork_choices',{gameId}).then(value=>{if(alive.current)setChoices(value);}).catch(error=>notify(String(error)));},[gameId]);
 useEffect(()=>()=>{if(draft)void invoke('discard_artwork',{draftId:draft.id}).catch(()=>undefined);},[draft?.id]);
 const current=draft||choices.find(choice=>choice.kind===kind);
 const choose=async()=>{setBusy(true);try{
  const path=await open({multiple:false,directory:false,filters:[{name:'图片',extensions:['png','jpg','jpeg','webp','gif']}]});if(!path)return;
  const next=await invoke<Draft>('prepare_artwork',{gameId,kind,path});
  if(!alive.current){await invoke('discard_artwork',{draftId:next.id});return;}setDraft(next);
 }catch(error){notify(String(error));}finally{if(alive.current)setBusy(false);}};
 const save=async()=>{if(!draft)return;setBusy(true);try{await invoke('commit_artwork',{gameId,kind,draftId:draft.id});setDraft(null);await refresh();notify('图片已应用');}catch(error){notify(String(error));}finally{if(alive.current)setBusy(false);}};
 const reset=async()=>{setBusy(true);try{await backend.resetCustomArtwork(gameId,kind);setDraft(null);await refresh();notify('已恢复自动素材');}catch(error){notify(String(error));}finally{if(alive.current)setBusy(false);}};
 return <section className="manual-artwork">
  <h3>手动指定图片</h3>
  {kind==='hero'&&<BackgroundCandidates key={gameId} gameId={gameId} onApplied={async()=>{setDraft(null);await refresh();}}/>}
  {(kind==='poster'||kind==='landscape')&&<MotionCardCandidates key={`${gameId}-${kind}`} gameId={gameId} kind={kind} onApplied={async()=>{setDraft(null);await refresh();}}/>}
  <div className="setting-buttons manual-art-sites"><span>找图网站</span>{[{id:'alphacoders',name:'Alpha Coders · 壁纸'},{id:'wallhaven',name:'Wallhaven · 壁纸'},{id:'steamgriddb',name:'SteamGridDB · 封面 / Logo'}].map(site=><button key={site.id} className="btn btn-text" onClick={()=>void invoke('open_artwork_website',{site:site.id}).catch(error=>notify(String(error)))}>{site.name} ↗</button>)}</div>
  <div className="manual-art-kinds" role="tablist" aria-label="图片用途">{kinds.map(item=><button key={item.id} role="tab" aria-selected={kind===item.id} disabled={busy} onClick={()=>{setDraft(null);setKind(item.id);setFill(false);}}>{item.name}</button>)}</div>
  <div className={`manual-art-preview manual-art-${kind}`}>
   {current?<MotionImage play={kind==='poster'||kind==='landscape'} src={convertFileSrc(current.path)} alt="选定图片预览" style={{objectFit:fill?'cover':'contain'}}/>:<span>尚未设置此类图片</span>}
  </div>
  <div className="manual-art-caption"><span>{current?`${current.width} × ${current.height}${kind==='hero'?(current.width>=3840&&current.height>=2160?' · 4K':' · 未达到 4K'):''}`:''}{draft?' · 待保存':choices.find(choice=>choice.kind===kind)?.manual?' · 手动指定':' · 自动素材'}</span>{kind!=='logo'&&<button className="btn btn-text" onClick={()=>setFill(value=>!value)}>{fill?'查看完整图片':'预览铺满效果'}</button>}</div>
  <div className="setting-buttons">
   <button className="btn btn-outline" disabled={busy} onClick={()=>void choose()}>选择图片 / 动图</button>
   <button className="btn btn-primary" disabled={busy||!draft} onClick={()=>void save()}>{busy?'处理中…':'保存并使用'}</button>
   {draft&&<button className="btn btn-text" disabled={busy} onClick={()=>setDraft(null)}>取消选图</button>}
   <button className="btn btn-text" disabled={busy||!choices.some(choice=>choice.kind===kind&&choice.manual)} onClick={()=>void reset()}>恢复自动图片</button>
  </div>
  <p className="setting-note">保存后复制到游戏大厅，原图片移动或删除不影响使用；后续刮削会保留手动图片。</p>
 </section>;
}
