import {EpicArtwork} from './EpicArtwork';
import {EpicAccount} from './EpicAccount';
import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {monitorId,useDisplayState,selectDisplay,setInterfaceScale,setFollowGame} from '../lib/display';
import { ArrowDownToLine, Gamepad2, RefreshCw, Keyboard } from 'lucide-react';
import { ManualArtwork } from './ManualArtwork';
import { platformName } from '../lib/platforms';
import { Select } from './Select';
import { backend, desktopMode, type SteamAccounts } from '../lib/backend';
import { useDeck } from '../lib/store';
import { syncWindowState, toggleFullscreen, useWindowState } from '../lib/window';
import { InputHints } from './InputHints';
import { useInputDevice } from '../lib/inputDevice';
import { playUiSound, setUiSoundsEnabled, uiSoundsEnabled } from '../lib/uiSounds';

export function Settings({ onScan, onImport }: { onScan: () => void; onImport: () => void }) {
  const inputMode = useInputDevice(s => s.mode);
  const allGames = useDeck(s => s.games);
  const games = useMemo(() => allGames.filter(game=>!game.hidden), [allGames]);
  const demo = useDeck(s => s.demo);
  const busy = useDeck(s => s.busy);
  const accentEnabled = useDeck(s => s.accentEnabled);
  const toggleFocusFx = useDeck(s => s.toggleFocusFx);
  const notify = useDeck(s => s.notify);
  const tabs = ['游戏库与账号','素材与刮削','显示','手柄与操作','关于'];
  const [tab, setTab] = useState(0);
  const [soundsEnabled, setSoundsEnabled] = useState(uiSoundsEnabled);
  const [motionEnabled,setMotionEnabled]=useState(()=>localStorage.getItem('gamedeck-motion')==='on');
  const [forceBorderless,setForceBorderless]=useState(true);
  const [alphaKey,setAlphaKey]=useState('');
  const [alphaConfigured,setAlphaConfigured]=useState(false);
  const [alphaSaving,setAlphaSaving]=useState(false);
  useEffect(()=>{void backend.alphaKeyStatus().then(setAlphaConfigured).catch(error=>notify(String(error)));},[]);
  const saveAlpha=async()=>{setAlphaSaving(true);try{await backend.setAlphaKey(alphaKey);setAlphaKey('');setAlphaConfigured(await backend.alphaKeyStatus());notify('Alpha Coders 凭据已更新');}catch(error){notify(String(error));}finally{setAlphaSaving(false);}};
  useEffect(() => {
    const change = (event: Event) => setTab(i => (i + (event as CustomEvent<number>).detail + tabs.length) % tabs.length);
    window.addEventListener('settings-tab',change);
    return () => window.removeEventListener('settings-tab',change);
  }, []);
  const [accounts,setAccounts] = useState<SteamAccounts>({accounts:[],selected:null,scope:'cache'});
  const [accountBusy,setAccountBusy] = useState(false);
  const [steamKey,setSteamKey] = useState('');
  useEffect(() => { void backend.steamAccounts().then(setAccounts).catch(error => notify(String(error))); }, []);
  const selectAccount = async (id:string) => {
    setAccountBusy(true);
    try { await backend.selectSteamAccount(id); setAccounts(await backend.steamAccounts()); useDeck.getState().setGames(await backend.listGames()); notify('Steam 账号已切换'); }
    catch (error) { notify(String(error)); } finally { setAccountBusy(false); }
  };
  const syncLibrary = async () => {
    setAccountBusy(true);
    try { const count = await backend.syncSteamLibrary(steamKey); setAccounts(await backend.steamAccounts()); useDeck.getState().setGames(await backend.listGames()); notify(`已同步账号库 ${count} 款游戏`); }
    catch (error) { notify(String(error)); } finally { setAccountBusy(false); }
  };
  const [candidateProvider,setCandidateProvider] = useState('steam');
  const [searchName,setSearchName] = useState('');
  const [scraping, setScraping] = useState(false);
  const [backgroundBusy,setBackgroundBusy]=useState(false);
  const [backgroundSummary,setBackgroundSummary]=useState<string[]>([]);
  const [syncingTime, setSyncingTime] = useState(false);
  const syncTime = async () => {
    setSyncingTime(true);
    try {
      const result = await backend.syncPlaytime();
      setAccounts(await backend.steamAccounts());
      useDeck.getState().setGames(await backend.listGames());
      notify(`已同步 ${result.updated} 款游戏的 Steam 时长`);
    } catch (error) { notify(String(error)); }
    finally { setSyncingTime(false); }
  };
  const [forceScrape, setForceScrape] = useState(false);
  const [scrapeResult, setScrapeResult] = useState<{processed:number;saved:number;errors:string[]}|null>(null);
  const [scrapeProgress, setScrapeProgress] = useState({processed:0,total:0,saved:0});
  const [igdbClientId, setIgdbClientId] = useState('');
  const [igdbToken, setIgdbToken] = useState('');
  const [selectedGame, setSelectedGame] = useState('');
  const [gameFilter,setGameFilter]=useState('');
  const [candidates, setCandidates] = useState<{id:number;name:string;exact:boolean}[]>([]);
  const [candidateBusy, setCandidateBusy] = useState(false);
  useEffect(() => {
    if (!desktopMode) return;
    let disposed = false; let unlisten: (()=>void)|undefined;
    void listen<{processed:number;total:number;saved:number}>('artwork-scrape-progress', e => setScrapeProgress(e.payload))
      .then(fn => { if (disposed) fn(); else unlisten = fn; });
    return () => { disposed = true; unlisten?.(); };
  }, []);
  const searchCandidates = async () => {
    const game = games.find(g => g.id === selectedGame); if (!game) return;
    setCandidateBusy(true);
    try { setCandidates(await backend.searchArtworkCandidates(searchName.trim() || game.name, apiKey, candidateProvider)); }
    catch (err) { notify(String(err)); }
    finally { setCandidateBusy(false); }
  };
  const selectCandidate = async (id: number) => {
    try { await backend.confirmArtworkMatch(selectedGame, id, candidateProvider); setForceScrape(true); notify('匹配已确认，下次刮削将更新缓存素材'); }
    catch (err) { notify(String(err)); }
  };
  const [apiKey, setApiKey] = useState('');
  const [sgdbConfigured,setSgdbConfigured]=useState(false);
  const [sgdbSaving,setSgdbSaving]=useState(false);
  useEffect(()=>{sessionStorage.removeItem('sgdb-key');void backend.sgdbKeyStatus().then(setSgdbConfigured).catch(e=>notify(String(e)));},[]);
  const saveSgdb=async(key:string)=>{setSgdbSaving(true);try{await backend.setSgdbKey(key);setApiKey('');setSgdbConfigured(await backend.sgdbKeyStatus());notify('SteamGridDB 凭据已更新');}catch(e){notify(String(e));}finally{setSgdbSaving(false);}};
  const scrape = async () => {
    setScraping(true);
    setScrapeResult(null);
    setScrapeProgress({processed:0,total:demo ? 0 : games.length,saved:0});
    try {
      const result = await backend.scrapeArtwork(apiKey, forceScrape, igdbClientId, igdbToken);
      setScrapeResult(result);
      useDeck.getState().setGames(await backend.listGames());
      notify(`刮削完成：处理 ${result.processed} 款，保存 ${result.saved} 张图片，${result.errors.length} 条错误。`);
    } catch (err) { notify(`刮削失败：${String(err)}`); }
    finally { setScraping(false); }
  };
  const refreshBackgrounds=async()=>{
    setBackgroundBusy(true);setBackgroundSummary([]);setScrapeProgress({processed:0,total:games.length,saved:0});
    try{const r=await backend.refreshPromoBackgrounds();useDeck.getState().setGames(await backend.listGames());
      setBackgroundSummary(r.entries.map(e=>{const reasons=[...new Set([...(e.audit?.issues||[]),...(e.audit?.candidates||[]).filter(c=>c.decision==='rejected').map(c=>c.reason).filter((s):s is string=>!!s)])];return `${e.game} · ${e.status==='manual-preserved'?'保留手动图':e.selection?`${e.selection.tier} · ${e.selection.width}×${e.selection.height}`:e.reason||e.status}${reasons.length?` · 未采用候选：${reasons.join('；')}`:''}`;}).concat(r.errors));
      notify(`已逐款核验 ${r.processed} 款，更新 ${r.saved} 款宣传背景`);
    }catch(e){notify(String(e));}finally{setBackgroundBusy(false);}
  };
  const {monitors,current,uiScale,followGame,zoom}=useDisplayState();
  const fullscreen = useWindowState(s => s.fullscreen);
  useEffect(() => {
    if (!desktopMode) return;
    void syncWindowState().catch(() => undefined);
    void invoke<{force_borderless:boolean}>('display_agent_config')
      .then(c=>{setForceBorderless(c.force_borderless!==false);}).catch(()=>undefined);
  }, []);
  const changeMonitor=async(id:string)=>{try{await selectDisplay(id);await syncWindowState();notify('已切换大厅显示器');}catch(error){notify(`切换显示器失败：${String(error)}`);}};
  type AgentCfg={enabled:boolean;primary_switch_on_game:boolean;force_borderless:boolean;[key:string]:unknown};
  const saveAgentCfg=async(patch:Partial<AgentCfg>)=>invoke<AgentCfg>('display_agent_config').then(async cfg=>invoke('save_display_agent_config',{config:{...cfg,...patch}}));
  const toggleBorderless=async()=>{const value=!forceBorderless;setForceBorderless(value);try{await saveAgentCfg({force_borderless:value});notify(value?'已开启：启动游戏前自动把已适配游戏设为无边框窗口化':'已关闭：不再改写游戏的全屏配置');}catch(error){setForceBorderless(!value);notify(`设置失败：${String(error)}`);}};
  const toggleFull = async () => {
    if (!desktopMode) return notify('浏览器预览无法控制原生全屏窗口');
    try { await toggleFullscreen(); }
    catch (err) { notify(`全屏切换失败：${String(err)}`); }
  };
  return <main className="settings page-in">
    <div className="settings-title"><h1>设置</h1><span>{games.length} 款游戏 · {games.filter(g=>g.installed).length} 款已安装</span></div>
    <nav className="settings-tabs" role="tablist" aria-label="设置分类">
      {tabs.map((name,i)=><button key={name} id={`setting-tab-${i}`} role="tab" aria-selected={tab===i} aria-controls={`setting-page-${i}`} tabIndex={tab===i?0:-1}
        onClick={()=>setTab(i)} onKeyDown={e=>{if(e.key==='ArrowRight'||e.key==='ArrowLeft'){e.preventDefault();e.stopPropagation();const next=(i+(e.key==='ArrowRight'?1:-1)+tabs.length)%tabs.length;setTab(next);document.getElementById(`setting-tab-${next}`)?.focus();}}}>{name}</button>)}
    </nav>
    <div className="settings-content" role="tabpanel" id={`setting-page-${tab}`} aria-labelledby={`setting-tab-${tab}`} key={tab}>
      {tab===0 && <>
        <h2>游戏来源</h2>
        <div className="setting-row"><div><h3>本机游戏</h3><p>Steam、Epic Games、Battle.net</p></div><div className="setting-buttons"><button className="btn btn-primary" onClick={onScan} disabled={busy||accountBusy}><RefreshCw size={17} className={busy?'spin':''}/>{busy?'扫描中…':'扫描本机'}</button><button className="btn btn-outline" onClick={onImport}><ArrowDownToLine size={17}/>导入 EXE</button></div></div>
        <div className="setting-row"><div><h3>Steam 账号</h3><p>时长与账号库分别保存，本机安装共用；启动使用 Steam 客户端当前账号。</p></div><Select label="Steam 账号" value={accounts.selected||''} disabled={busy||accountBusy||!accounts.accounts.length} onChange={id=>void selectAccount(id)} options={[...(!accounts.selected?[{value:'',label:'选择账号'}]:[]),...accounts.accounts.map(a=>({value:a.id,label:`${a.name}${a.most_recent?' · 最近登录':''}`}))]}/></div>
        <div className="setting-row"><div><h3>Steam 本地缓存</h3><p>{games.filter(g=>g.playtime_source==='steam-local').length} 款已有时长 · {accounts.scope==='owned'?'已同步完整账号库':'含本机已安装与缓存中有游玩记录的游戏，非完整已购库'}</p></div><button className="btn btn-outline" onClick={()=>void syncTime()} disabled={!desktopMode||syncingTime||accountBusy||busy}>{syncingTime?'同步中…':'同步缓存与时长'}</button></div>
        <EpicAccount/><div className="setting-row"><div><h3>Battle.net 账号</h3><p>从本机安装清单导入，启动时使用战网客户端当前登录的账号。</p></div><span className="setting-value">本机安装</span></div>
        <details className="setting-advanced"><summary>同步 Steam 完整已购库</summary><p className="setting-note">需要 Steam Web API Key，且账号的游戏详情可公开读取。密钥仅用于本次请求。</p><div className="setting-inline"><input type="password" className="scraper-field" aria-label="Steam Web API Key" placeholder="Steam Web API Key" value={steamKey} onChange={e=>setSteamKey(e.target.value)}/><button className="btn btn-primary" disabled={!desktopMode||!steamKey.trim()||!accounts.selected||accountBusy||busy} onClick={()=>void syncLibrary()}>{accountBusy?'同步中…':'同步全部已购游戏'}</button></div></details>
      </>}
      {tab===1 && <>
        <h2>游戏素材</h2>
          <EpicArtwork/>
        <div className="setting-row"><div><h3>自动刮削</h3><p>优先本地缓存；非 Steam 游戏可按名称匹配 Steam 素材，无需密钥。</p></div><button className="btn btn-primary" onClick={()=>void scrape()} disabled={scraping||backgroundBusy||!desktopMode}>{scraping?`刮削中 ${scrapeProgress.processed}/${scrapeProgress.total}`:'开始刮削'}</button></div>
        <div className="setting-row"><div><h3>重新下载自动素材</h3><p>手动选择的图片继续保留。</p></div><button className={`toggle ${forceScrape?'on':''}`} role="switch" aria-label="重新下载自动素材" aria-checked={forceScrape} disabled={scraping} onClick={()=>setForceScrape(v=>!v)}><span/></button></div>
        {scrapeResult && <div className="scrape-status" role="status"><p>已处理 {scrapeResult.processed} 款 · 缓存 {scrapeResult.saved} 张 · {scrapeResult.errors.length} 条错误</p>{scrapeResult.errors.length>0&&<details><summary>查看失败原因</summary><ul>{scrapeResult.errors.map((error,i)=><li key={i}>{error}</li>)}</ul></details>}</div>}
        <div className="setting-row"><div><h3>整库宣传背景</h3><p>重审旧缓存；优先 4K / 2K 宣传主视觉，无合格图时回退官方横幅，保留手动图。</p></div><button className="btn btn-outline" disabled={scraping||backgroundBusy||!desktopMode} onClick={()=>void refreshBackgrounds()}>{backgroundBusy?`核验中 ${scrapeProgress.processed}/${scrapeProgress.total}`:'更新整库宣传背景'}</button></div>
        {backgroundSummary.length>0&&<details className="scrape-status"><summary>逐款核验结果（{backgroundSummary.length}）</summary><ul>{backgroundSummary.map((text,i)=><li key={i}>{text}</li>)}</ul></details>}
        <h2>单款游戏</h2>
        <input className="scraper-field" aria-label="筛选要换图的游戏" placeholder="输入游戏名或平台筛选" value={gameFilter} onChange={event=>setGameFilter(event.target.value)}/>

        <div className="setting-row"><h3>选择游戏</h3><Select label="选择游戏" value={selectedGame} onChange={id=>{setSelectedGame(id);setSearchName(games.find(g=>g.id===id)?.name||'');setCandidates([]);}} options={[{value:'',label:'选择游戏'},...games.filter(g=>!g.id.startsWith('demo-')&&(g.id===selectedGame||`${g.name} ${g.original_name||''} ${platformName(g.platform)}`.toLowerCase().includes(gameFilter.toLowerCase()))).map(g=>({value:g.id,label:`${g.name} · ${platformName(g.platform)}`}))]}/></div>
        {selectedGame&&<ManualArtwork key={selectedGame} gameId={selectedGame}/>}
        <div className="setting-row"><h3>匹配来源</h3><Select label="匹配来源" value={candidateProvider} onChange={value=>{setCandidateProvider(value);setCandidates([]);}} options={[{value:'steam',label:'Steam 商店 · 无需密钥'},{value:'steamgriddb',label:'SteamGridDB'}]}/></div>
        <div className="setting-inline"><input aria-label="素材搜索名称" placeholder="搜索名称，可填写中文或英文名" className="scraper-field" value={searchName} onChange={e=>setSearchName(e.target.value)}/><button className="btn btn-outline" disabled={!selectedGame||candidateBusy||(candidateProvider==='steamgriddb'&&!apiKey&&!sgdbConfigured)} onClick={()=>void searchCandidates()}>{candidateBusy?'搜索中…':'搜索素材'}</button></div>
        {candidates.map(c=><div key={c.id} className="setting-row"><span>{c.name}{c.exact?' · 精确匹配':''}</span><button className="btn btn-outline" onClick={()=>void selectCandidate(c.id)}>确认匹配</button></div>)}
        <div className="setting-row"><span>Alpha Coders · {alphaConfigured?'已配置':'未配置'}</span></div>
        <label>Alpha Coders 私人 Key<input type="password" autoComplete="off" value={alphaKey} onChange={e=>setAlphaKey(e.target.value)} className="scraper-field" placeholder="Key 使用 Windows 账号加密保存"/></label>
        <button className="btn btn-outline" disabled={alphaSaving||!alphaKey.trim()} onClick={()=>void saveAlpha()}>保存 Key</button>
        <button className="btn btn-outline" disabled={alphaSaving||!alphaConfigured} onClick={()=>{setAlphaSaving(true);void backend.setAlphaKey('').then(()=>setAlphaConfigured(false)).catch(e=>notify(String(e))).finally(()=>setAlphaSaving(false));}}>清除 Key</button>
        <details className="setting-advanced"><summary>其他素材服务（可选）</summary><p className="setting-note">SteamGridDB Key 使用 Windows 账号加密保存；IGDB 凭据仅用于当前会话。</p>
          <label>SteamGridDB API Key<input type="password" aria-label="SteamGridDB API Key" value={apiKey} autoComplete="off" placeholder={sgdbConfigured?'已配置，输入可更换':'未配置'} onChange={e=>setApiKey(e.target.value)} className="scraper-field"/></label>
          <button className="btn btn-outline" disabled={sgdbSaving||!apiKey.trim()} onClick={()=>void saveSgdb(apiKey)}>保存 SteamGridDB Key</button>
          <button className="btn btn-outline" disabled={sgdbSaving||!sgdbConfigured} onClick={()=>void saveSgdb('')}>清除 SteamGridDB Key</button>
          <label>IGDB Client ID<input aria-label="IGDB Client ID" value={igdbClientId} onChange={e=>setIgdbClientId(e.target.value)} className="scraper-field"/></label>
          <label>IGDB Access Token<input type="password" aria-label="IGDB Access Token" value={igdbToken} onChange={e=>setIgdbToken(e.target.value)} className="scraper-field"/></label>
        </details>
      </>}
      {tab===2 && <>
        <h2>显示与画面</h2>
        <div className="setting-row"><div><h3>动态图</h3><p>默认关闭。仅竖封面和横卡片选中时播放，最长 8 秒；大背景保持静态。</p></div><button className={`toggle ${motionEnabled?'on':''}`} role="switch" aria-label="动态图" aria-checked={motionEnabled} onClick={()=>{const value=!motionEnabled;setMotionEnabled(value);localStorage.setItem('gamedeck-motion',value?'on':'off');window.dispatchEvent(new Event('gamedeck-motion'));}}><span/></button></div>
        <div className="setting-row"><h3>全屏显示</h3><button aria-label="全屏显示" className={`toggle ${fullscreen?'on':''}`} onClick={toggleFull} role="switch" aria-checked={fullscreen}><span/></button></div>
        <div className="setting-row"><div><h3>大厅显示器</h3><p>默认按 Windows 的显示器设置启动（电视与 1 屏复制时画面相同）；这里可手动指定大厅所在显示器。</p></div><Select label="大厅显示器" value={current} options={monitors.length?monitors.map((m,i)=>({value:monitorId(m),label:`${m.name||`显示器 ${i+1}`} · ${m.size.width} × ${m.size.height}${monitorId(m)===current?' · 当前':''}`})):[{value:'',label:'未检测到显示器'}]} onChange={value=>void changeMonitor(value)} disabled={!monitors.length}/></div>
        <div className="setting-row"><div><h3>游戏跟随大厅屏幕</h3><p>启动时把游戏窗口移到大厅所在的显示器；部分独占全屏游戏还需在游戏内选择屏幕。</p></div><button className={`toggle ${followGame?'on':''}`} role="switch" aria-label="游戏跟随大厅屏幕" aria-checked={followGame} onClick={()=>setFollowGame(!followGame)}><span/></button></div>
        <div className="setting-row"><div><h3>游戏无边框启动（已适配游戏）</h3><p>启动前改写游戏自己的显示配置，让游戏以无边框窗口化模式启动——这是游戏本身的设置，不走外部抠窗口，画面不会糊。目前已适配影之刃零、幽灵行者、剑星、无主之地3/4 等 UE 游戏；其它引擎需在游戏中手动设置。每次启动前自动写入并备份原配置。</p></div><button className={`toggle ${forceBorderless?'on':''}`} role="switch" aria-label="游戏无边框启动" aria-checked={forceBorderless} onClick={()=>void toggleBorderless()}><span/></button></div>
        <div className="setting-row"><div><h3>界面缩放</h3><p>自动适配当前屏幕的 Windows 缩放，当前 {Math.round(zoom*100)}%。</p></div><Select label="界面缩放" value={uiScale} options={[{value:'auto',label:'自动适配'},...['0.8','0.9','1','1.1','1.2'].map(value=>({value,label:`${Math.round(Number(value)*100)}%`}))]} onChange={value=>void setInterfaceScale(value).catch(error=>notify(`缩放失败：${String(error)}`))}/></div>
      </>}
      {tab===3 && <>
        <h2>手柄与操作</h2>
        <div className="setting-row"><div><h3>操作音效</h3><p>切换游戏时轻响，发出启动请求时播放确认音。关闭后保持安静。</p></div><button className={`toggle ${soundsEnabled?'on':''}`} role="switch" aria-label="操作音效" aria-checked={soundsEnabled} onClick={()=>{const value=!soundsEnabled;setUiSoundsEnabled(value);setSoundsEnabled(value);}}><span/></button></div>
        <div className="setting-row"><h3>音效试听</h3><div className="setting-buttons"><button className="btn btn-outline" disabled={!soundsEnabled} onClick={()=>playUiSound('select')}>试听切换</button><button className="btn btn-outline" disabled={!soundsEnabled} onClick={()=>playUiSound('launch')}>试听启动</button></div></div>
        <div className="setting-row"><h3>高亮焦点动画</h3><button aria-label="高亮焦点动画" className={`toggle ${accentEnabled?'on':''}`} onClick={toggleFocusFx} role="switch" aria-checked={accentEnabled}><span/></button></div>
        <div className="setting-row"><h3>当前输入设备</h3><span className="setting-value">{inputMode==='gamepad'?<Gamepad2 size={18}/>:<Keyboard size={18}/>} {inputMode==='gamepad'?'手柄':'键盘与鼠标'}</span></div>
        <div className="setting-row"><h3>操作提示</h3><span className="setting-value">随实际输入自动切换</span></div>
      </>}
      {tab===4 && <><h2>GameDeck</h2><div className="setting-row"><h3>版本</h3><span className="setting-value">0.6</span></div><div className="setting-row"><h3>运行模式</h3><span className="setting-value">{desktopMode?'Windows':'浏览器预览'}</span></div></>}
    </div>
    <footer className="settings-footer"><InputHints context="settings"/></footer>
  </main>;
}
