import { useMemo, useState } from 'react';
import { backend } from '../lib/backend';
import { useDeck } from '../lib/store';
import { DeckDialog } from './DeckDialog';
import { platformName } from '../lib/platforms';

export function MoveToHidden({onClose}:{onClose:()=>void}) {
  const allGames=useDeck(state=>state.games);
  const [query,setQuery]=useState('');
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const games=useMemo(()=>allGames.filter(game=>!game.hidden&&`${game.name} ${game.original_name||''}`.toLowerCase().includes(query.trim().toLowerCase())),[allGames,query]);
  const move=async(id:string)=>{
    setBusy(true);setError('');
    try{await backend.setHidden(id,true);const state=useDeck.getState();state.updateGame(id,{hidden:true});state.setPlatformFilter('all');state.setFilter('all');state.setQuery('');state.setFocus(id);state.notify('已移入隐藏空间');}
    catch(error){setError(String(error));}finally{setBusy(false);}
  };
  return <DeckDialog title="从游戏库移入" onClose={onClose} busy={busy}>
    <p className="dialog-note">移入后将从大厅和普通游戏库中隐藏，收藏、图片与游玩记录会保留。</p>
    <label className="dialog-field">搜索游戏<input value={query} disabled={busy} onChange={event=>setQuery(event.target.value)}/></label>
    <div className="hidden-move-list">{games.map(game=><button className="hidden-move-row" key={game.id} disabled={busy} onClick={()=>void move(game.id)}><span>{game.name}<small>{platformName(game.platform)}</small></span><span>移入</span></button>)}{games.length===0&&<p className="dialog-note">没有符合条件的游戏。</p>}</div>
    {error&&<p className="dialog-error" role="alert">{error}</p>}
    <div className="dialog-actions"><button className="btn btn-outline" disabled={busy} onClick={onClose}>完成</button></div>
  </DeckDialog>;
}
