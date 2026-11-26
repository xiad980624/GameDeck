import {LaunchButton} from './LaunchButton';
import {GameTitle} from './GameTitle';
import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { backend, desktopMode } from '../lib/backend';
import { ArrowLeft, Heart, Clock3, HardDrive, MonitorPlay, Bookmark } from 'lucide-react';
import { motion } from 'motion/react';
import { useDeck } from '../lib/store';
import { upgradeHero } from '../lib/artwork';
import { HeroImage } from './HeroImage';
import { Select } from './Select';
import { gameAccelerator, saveGameAccelerator, type AcceleratorId } from '../lib/acceleration';
import { InputHints } from './InputHints';
import { ManualArtwork } from './ManualArtwork';
import type { Game } from '../lib/types';
import { platformName,updateLabel } from '../lib/platforms';

export function Details({ onLaunch, onFavorite }: { onLaunch: (game: Game) => void; onFavorite: (game: Game) => void }) {
  const id = useDeck(s => s.detailId);
  const game = useDeck(s => s.games.find(g => g.id === id));
  const back = useDeck(s => s.closeDetail);
  const hiddenSpace = useDeck(s => s.hiddenSpace);
  const [moving,setMoving] = useState(false);
  const [accelerator,setAccelerator]=useState<AcceleratorId|''>('');
  useEffect(()=>setAccelerator(id?gameAccelerator(id):''),[id]);
  const [upgrading,setUpgrading]=useState(false);
  const [screenshots, setScreenshots] = useState<string[]>([]);
  useEffect(() => {
    if (!id) return;
    let active = true;
    void backend.listGameScreenshots(id).then(paths => { if (active) setScreenshots(paths); }).catch(() => undefined);
    return () => { active = false; };
  }, [id]);
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
      <p className="details-genres">{game.genre} <span>·</span> {platformName(game.platform)}</p>
      {game.description && <p className="details-desc">{game.description}</p>}
      <div className="details-actions">
        <LaunchButton game={game} onLaunch={onLaunch}/>
        <button className="btn btn-glass" onClick={() => onFavorite(game)}><Heart size={18} fill={game.favorite ? 'currentColor' : 'none'} /> {game.favorite ? '已收藏' : '加入收藏'}</button>
        {hiddenSpace&&<button className="btn btn-outline" disabled={moving} onClick={()=>{setMoving(true);void backend.setHidden(game.id,false).then(()=>{useDeck.getState().updateGame(game.id,{hidden:false});back();useDeck.getState().notify('已移回普通游戏库');}).catch(error=>useDeck.getState().notify(String(error))).finally(()=>setMoving(false));}}>移回游戏库</button>}
        {['steam','epic','battlenet'].includes(game.platform)&&<button className="btn btn-outline" onClick={()=>void backend.openPlatform(game.platform,'updates').catch(error=>useDeck.getState().notify(String(error)))}>打开客户端检查更新</button>}
        {<button className="btn btn-outline" disabled={upgrading} onClick={()=>{setUpgrading(true);void upgradeHero(game,true).then(()=>useDeck.getState().notify('已按宣传主视觉规则更新背景')).catch(error=>useDeck.getState().notify(String(error))).finally(()=>setUpgrading(false));}}>{upgrading?'正在获取原图…':'更新宣传背景'}</button>}
      </div>
      <div className="detail-accelerator"><span>启动前加速</span><Select label="启动前加速" value={accelerator} onChange={value=>{setAccelerator(value);saveGameAccelerator(game.id,value);}} options={[{value:'',label:'不使用'},{value:'uu',label:'UU加速器'},{value:'xunyou',label:'迅游加速器'}]}/><button className="btn btn-outline" onClick={()=>useDeck.getState().openAccelerators(accelerator||'uu',game.id)}>打开加速器页</button></div>
      <div className="detail-info-grid">
        {!!game.hero_width&&!!game.hero_height&&<div><MonitorPlay size={19}/><span>背景原图</span><strong>{game.hero_width} × {game.hero_height}</strong><small>{game.hero_source?.startsWith('background-')?'背景专用素材':game.hero_source==='steam-local'?'Steam 本地缓存':game.hero_source==='epic-local'?'Epic 本地缓存':game.hero_source==='battlenet-local'?'战网本地缓存':'导入 / 刮削素材'}</small></div>}
        <div><Clock3 size={19} /><span>{game.playtime_source === 'steam-local' ? 'Steam 时长' : 'GameDeck 记录'}</span><strong>{game.total_minutes || game.playtime_source === 'steam-local' ? `${(game.total_minutes / 60).toFixed(1)} 小时` : '无记录'}</strong></div>
        <div><HardDrive size={19} /><span>安装状态</span><strong>{game.installed ? '已安装' : '未安装'}</strong></div>
        <div><MonitorPlay size={19} /><span>游戏平台</span><strong>{platformName(game.platform)}</strong></div>
        <div><Bookmark size={19} /><span>收藏状态</span><strong>{game.favorite ? '我的收藏' : '未收藏'}</strong></div>
        <div title={game.update_detail||''}><HardDrive size={19}/><span>更新状态</span><strong>{updateLabel(game)}</strong></div>
        {game.installed_version&&<div><Bookmark size={19}/><span>已安装版本</span><strong>{game.installed_version}</strong></div>}
      </div>
      <div className="details-input-hints"><InputHints context="details"/></div>
      {hiddenSpace&&<ManualArtwork key={game.id} gameId={game.id}/>}
      {screenshots.length > 0 && <section style={{marginTop:24}}><h3>游戏截图</h3><div style={{display:'flex',gap:12,overflowX:'auto',paddingBottom:12}}>{screenshots.map(path=><img loading="lazy" decoding="async" fetchPriority="low" key={path} src={desktopMode ? convertFileSrc(path) : path} alt={`${game.name} 截图`} style={{width:320,aspectRatio:'16/9',objectFit:'cover',borderRadius:10}} />)}</div></section>}
    </motion.div>
  </main>;
}
