import {useEffect,useState} from 'react';
import {RefreshCw,ExternalLink} from 'lucide-react';
import {backend,desktopMode,type EpicAccountStatus,type EpicLibrarySync} from '../lib/backend';
import {useDeck} from '../lib/store';

const empty:EpicAccountStatus={connected:false,display_name:'',count:0,synced_at:null,syncing:false};
export function EpicAccount(){
 const [status,setStatus]=useState(empty);const [working,setWorking]=useState(false);const [expanded,setExpanded]=useState(false);const [code,setCode]=useState('');const [message,setMessage]=useState('');const [error,setError]=useState('');
 const globalBusy=useDeck(s=>s.busy);const disabled=working||globalBusy||!desktopMode;
 const refresh=async()=>{const next=await backend.epicAccountStatus();setStatus(next);useDeck.getState().setGames(await backend.listGames());};
 useEffect(()=>{void backend.epicAccountStatus().then(setStatus).catch(error=>setError(String(error)));},[]);
 const done=(result:EpicLibrarySync)=>setMessage(`账号库已同步 ${result.count} 款游戏 · 新增 ${result.inserted} 款${result.unresolved?` · ${result.unresolved} 项暂未确认，已保留原有游戏`:' · 包含未安装游戏'}`);
 const login=async()=>{setError('');setExpanded(true);try{await backend.openEpicLogin();}catch(e){setError(String(e));}};
 const connect=async()=>{setWorking(true);setError('');setMessage('');try{done(await backend.connectEpicAccount(code));setCode('');setExpanded(false);}catch(e){setError(String(e));}finally{await refresh().catch(()=>undefined);setWorking(false);}};
 const sync=async()=>{setWorking(true);setError('');setMessage('');try{done(await backend.syncEpicLibrary());}catch(e){setError(String(e));}finally{await refresh().catch(()=>undefined);setWorking(false);}};
 const disconnect=async()=>{setWorking(true);setError('');try{await backend.disconnectEpicAccount();setCode('');setMessage('已断开 Epic 账号；本机已安装游戏仍保留。');await refresh();}catch(e){setError(String(e));}finally{setWorking(false);}};
 return <section className="epic-account" aria-label="Epic 账号游戏库">
  <div className="setting-row"><div><h3>Epic 账号</h3><p>{status.connected?`${status.display_name||'已连接'} · 已同步 ${status.count} 款账号游戏`:'连接后导入全部已拥有游戏，包含未安装游戏。启动与安装仍由 Epic 客户端处理。'}</p>{status.synced_at&&<p>最近同步：{new Date(status.synced_at).toLocaleString()}</p>}</div>
   <div className="setting-buttons">{status.connected?<><button className="btn btn-outline" disabled={disabled} onClick={()=>void sync()}><RefreshCw size={17} className={working?'spin':''}/>{working?'同步中…':'同步账号库'}</button><button className="btn btn-outline" disabled={disabled} onClick={()=>void disconnect()}>断开</button></>:null}<button className="btn btn-outline" disabled={disabled} onClick={()=>void login()}><ExternalLink size={16}/>{status.connected?'重新连接':'连接 Epic 账号'}</button></div>
  </div>
  {expanded&&<div className="setting-advanced epic-login"><p className="setting-note">在 Epic 官方网页完成登录，将页面返回的一次性登录码或整段登录结果粘贴到这里。连接信息使用当前 Windows 账号加密保存。</p><div className="setting-inline"><input type="password" autoComplete="off" className="scraper-field" aria-label="Epic 一次性登录码" placeholder="一次性登录码或登录结果" value={code} onChange={e=>setCode(e.target.value)}/><button className="btn btn-primary" disabled={disabled||!code.trim()} onClick={()=>void connect()}>{working?'连接并同步中…':'连接并同步游戏库'}</button><button className="btn btn-outline" disabled={working} onClick={()=>{setExpanded(false);setCode('');}}>取消</button></div></div>}
  {message&&<p className="setting-note" role="status">{message}</p>}{error&&<p className="launch-error" role="alert">{error}</p>}
 </section>;
}
