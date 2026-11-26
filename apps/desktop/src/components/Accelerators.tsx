import { useEffect, useState } from 'react';
import { Zap, ExternalLink, FolderOpen, RefreshCw, Play } from 'lucide-react';
import { backend, type Accelerator } from '../lib/backend';
import { useDeck } from '../lib/store';
import { saveGameAccelerator } from '../lib/acceleration';
import type { Game } from '../lib/types';
import { InputHints } from './InputHints';

export function Accelerators({onContinue}:{onContinue:(game:Game)=>void}){
  const provider=useDeck(s=>s.acceleratorProvider);
  const setProvider=useDeck(s=>s.setAcceleratorProvider);
  const game=useDeck(s=>s.games.find(g=>g.id===s.acceleratorGameId));
  const busy=useDeck(s=>s.busy);
  const [clients,setClients]=useState<Accelerator[]>([]);
  const [loading,setLoading]=useState(true);
  const [opening,setOpening]=useState(false);
  const [opened,setOpened]=useState<string|null>(null);
  const refresh=async()=>{setLoading(true);try{setClients(await backend.accelerators());}catch(error){useDeck.getState().notify(String(error));}finally{setLoading(false);}};
  useEffect(()=>{void refresh();},[]);
  const client=clients.find(c=>c.id===provider);
  const open=async(website=false)=>{setOpening(true);try{await backend.openAccelerator(provider,website);if(!website){setOpened(provider);useDeck.getState().notify(`已打开${client?.name||'加速器'}`);}}catch(error){useDeck.getState().notify(String(error));}finally{setOpening(false);}};
  return <main className="accelerators page-in">
    <div className="library-heading"><h1>加速器</h1></div>
    <div className="platform-tabs accelerator-tabs" role="tablist" aria-label="加速器" onKeyDown={event=>{if(['ArrowLeft','ArrowRight','Home','End'].includes(event.key)){event.preventDefault();event.stopPropagation();const next=event.key==='Home'?'uu':event.key==='End'?'xunyou':provider==='uu'?'xunyou':'uu';setProvider(next);document.getElementById(`accelerator-${next}`)?.focus();}}}>
      {(['uu','xunyou'] as const).map(id=><button id={`accelerator-${id}`} key={id} role="tab" aria-selected={provider===id} aria-controls="accelerator-panel" tabIndex={provider===id?0:-1} onClick={()=>setProvider(id)}>{id==='uu'?'UU加速器':'迅游加速器'}</button>)}
    </div>
    <section id="accelerator-panel" role="tabpanel" aria-labelledby={`accelerator-${provider}`}>
      <div className="accelerator-title"><Zap size={32}/><div><h2>{provider==='uu'?'UU加速器':'迅游加速器'}</h2><span className={client?.path?'accelerator-installed':''}>{loading?'正在检测客户端…':client?.path?'已安装':'未检测到客户端'}</span></div></div>
      {game&&<p className="accelerator-game">准备启动：<strong>{game.name}</strong></p>}
      <p className="accelerator-instruction">在加速器客户端选择游戏和区服，开启加速后返回{game?'启动游戏':'游戏库'}。</p>
      {client?.path&&<div className="accelerator-location"><span>程序位置</span><code>{client.path}</code></div>}
      <div className="accelerator-actions">
        <button className="btn btn-primary" disabled={loading||opening||!client?.path} onClick={()=>void open()}><Zap size={18}/>打开加速器</button>
        <button className="btn btn-outline" disabled={opening} onClick={()=>void open(true)}><ExternalLink size={17}/>官网下载</button>
        <button className="btn btn-outline" disabled={loading||opening} onClick={()=>void backend.chooseAccelerator(provider).then(changed=>{if(changed)void refresh();}).catch(error=>useDeck.getState().notify(String(error)))}><FolderOpen size={17}/>指定程序</button>
        <button className="btn btn-outline" disabled={loading} onClick={()=>void refresh()}><RefreshCw size={17}/>重新检测</button>
      </div>
      {opened===provider&&<p className="accelerator-note" role="status">客户端已打开，请在客户端确认加速状态。</p>}
      <div className="accelerator-continue">
        {game?<><button className="btn btn-primary" disabled={busy||opening} onClick={()=>onContinue(game)}><Play size={18}/>启动游戏</button><button className="btn btn-outline" onClick={()=>{saveGameAccelerator(game.id,provider);useDeck.getState().notify(`已设置 ${game.name} 启动前使用${client?.name||'此加速器'}`);}}>设为此游戏默认加速器</button></>:<button className="btn btn-outline" onClick={()=>useDeck.getState().setPage('library')}>返回游戏库</button>}
      </div>
    </section>
    <footer className="accelerator-footer"><InputHints context="accelerators"/></footer>
  </main>;
}
