import {LaunchButton} from './LaunchButton';
import {GameTitle} from './GameTitle';
import { useEffect, useState } from 'react';
import { backend, desktopMode } from '../lib/backend';
import { ArrowLeft, Heart, MoreHorizontal } from 'lucide-react';
import { motion } from 'motion/react';
import { useDeck } from '../lib/store';
import { HeroImage } from './HeroImage';
import { gameAccelerator, type AcceleratorId } from '../lib/acceleration';
import { InputHints } from './InputHints';
import { DetailsMore, gamePlaytime } from './DetailsMore';
import type { Game } from '../lib/types';
import { platformName, updateLabel, updatePending } from '../lib/platforms';

export function Details({ onLaunch, onFavorite }: { onLaunch: (game: Game) => void; onFavorite: (game: Game) => void }) {
  const id = useDeck(s => s.detailId);
  const game = useDeck(s => s.games.find(g => g.id === id));
  const back = useDeck(s => s.closeDetail);
  const hiddenSpace = useDeck(s => s.hiddenSpace);
  const [moreOpen, setMoreOpen] = useState(false);
  const [accelerator,setAccelerator]=useState<AcceleratorId|''>('');
  useEffect(()=>{setAccelerator(id?gameAccelerator(id):'');setMoreOpen(false);},[id]);
  useEffect(()=>{
    if(!id||!desktopMode)return;
    let active=true,pending=false;
    const refresh=async()=>{
      if(!active||pending||document.visibilityState==='hidden')return;
      pending=true;
      try{
        const fresh=await backend.refreshGameStatus(id);
        if(active&&fresh)useDeck.getState().updateGame(id,{
          update_status:fresh.update_status,installed_version:fresh.installed_version,
          update_detail:fresh.update_detail,update_checked_at:fresh.update_checked_at,
        });
      }catch{/* Preserve the last known state on transient client or database read failures. */}
      finally{pending=false;}
    };
    void refresh();
    const timer=window.setInterval(()=>void refresh(),15000);
    const resumed=()=>void refresh();
    window.addEventListener('focus',resumed);
    document.addEventListener('visibilitychange',resumed);
    return()=>{active=false;clearInterval(timer);window.removeEventListener('focus',resumed);document.removeEventListener('visibilitychange',resumed);};
  },[id]);
  if (!game) return null;
  return <main className="details page-in">
    <div className="details-backdrop"><HeroImage game={game}/><div className="details-overlay" /></div>
    <button className="detail-back" onClick={back}><ArrowLeft size={20}/> {hiddenSpace?'返回隐藏空间':'返回游戏库'}</button>
    <motion.div className="details-body" initial={{ opacity: 0, y: 14 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: .28 }}>
      <GameTitle game={game}/>
      <p className="details-genres">{game.genre} <span>·</span> {platformName(game.platform)} <span>·</span> {gamePlaytime(game)}</p>
      {game.description && <p className="details-desc">{game.description}</p>}
      <div className="details-actions">
        <LaunchButton game={game} onLaunch={onLaunch}/>
        <button className="btn btn-square" aria-label={game.favorite?'取消收藏':'收藏游戏'} title={game.favorite?'取消收藏':'收藏游戏'} aria-pressed={game.favorite} onClick={() => onFavorite(game)}><Heart size={20} fill={game.favorite ? 'currentColor' : 'none'} /></button>
        <button className="btn btn-glass" data-detail-more aria-haspopup="dialog" aria-expanded={moreOpen} onClick={()=>setMoreOpen(true)}><MoreHorizontal size={20}/>更多</button>
      </div>
      {(accelerator||updatePending(game))&&<div className="details-notices">
        {accelerator&&<span>启动前使用{accelerator==='uu'?'UU加速器':'迅游加速器'}</span>}
        {updatePending(game)&&<span className="detail-update" role="status" title={game.update_detail||''}>{updateLabel(game)}</span>}
      </div>}
      <div className="details-input-hints"><InputHints context="details"/></div>
    </motion.div>
    {moreOpen&&<DetailsMore key={game.id} game={game} hiddenSpace={hiddenSpace} accelerator={accelerator} onAcceleratorChange={setAccelerator} onClose={()=>setMoreOpen(false)}/>}
  </main>;
}
