import { lazy, Suspense, useCallback, useEffect, useRef, useState } from 'react';
import { AlertCircle, CheckCircle2, X, LoaderCircle } from 'lucide-react';
const Accelerators=lazy(()=>import('./components/Accelerators').then(module=>({default:module.Accelerators})));
import { homeGames } from './lib/homeOrder';
import { gameAccelerator } from './lib/acceleration';
import { Header } from './components/Header';
import { Home } from './components/Home';
const Library=lazy(()=>import('./components/Library').then(module=>({default:module.Library})));
const Details=lazy(()=>import('./components/Details').then(module=>({default:module.Details})));
const Settings=lazy(()=>import('./components/Settings').then(module=>({default:module.Settings})));
import { backend, desktopMode } from './lib/backend';
import { useDeck } from './lib/store';
import { useDeckInput } from './lib/input';
import type { Game, InputAction, Page } from './lib/types';
import { useInputDevice } from './lib/inputDevice';
import { libraryPlatforms } from './lib/platforms';
import { libraryGames as visibleGames, spaceGames, hiddenShortcut } from './lib/libraryScope';
import { inputLabels } from './lib/inputPrompts';

import { primeHeroPaths } from './lib/artwork';
import { markStartup } from './lib/startup';
import {launchPending} from './lib/launchState';
import {initializeDisplay} from './lib/display';

export default function App() {
  useEffect(()=>{let disposed=false;let cleanup:(()=>void)|undefined;void initializeDisplay().then(fn=>{if(disposed)fn();else cleanup=fn;}).catch(error=>useDeck.getState().notify(`显示设置未应用：${String(error)}`));return()=>{disposed=true;cleanup?.();};},[]);
  const libraryLoaded=useDeck(s=>s.libraryLoaded);
  const [startupError,setStartupError]=useState('');
  const probeRef=useRef(false);
  const backgroundScanning=useRef(false);
  const [backgroundStatus,setBackgroundStatus]=useState<{label:string;running:boolean}|null>(null);
  const backgroundStatusTimer=useRef<number|undefined>(undefined);
  useEffect(()=>()=>clearTimeout(backgroundStatusTimer.current),[]);
  const lastBackgroundScan=useRef(Number(localStorage.getItem('gamedeck-last-scan')||0));
  useEffect(()=>{
    const timer=setTimeout(()=>markStartup('shell'),120);
    const frame=requestAnimationFrame(()=>requestAnimationFrame(()=>markStartup('shell')));
    return()=>{clearTimeout(timer);cancelAnimationFrame(frame);};
  },[]);
  useEffect(()=>{if(libraryLoaded)markStartup('library');},[libraryLoaded]);
  const page = useDeck(s => s.page);
  const previousPage=useDeck(s=>s.previousPage);
  const toast = useDeck(s => s.toast);
  const busy = useDeck(s => s.busy);
  const busyLabel = useDeck(s => s.busyLabel);
  const accentEnabled = useDeck(s => s.accentEnabled);
  const hiddenSpace = useDeck(s => s.hiddenSpace);

  useEffect(() => {
    const onShortcut = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const editing = Boolean(document.querySelector('[role="dialog"]') || target?.closest('input,textarea,select,[contenteditable]:not([contenteditable="false"])'));
      if (!hiddenShortcut(event, useDeck.getState().page, editing)) return;
      event.preventDefault(); event.stopImmediatePropagation();
      useDeck.getState().toggleHiddenSpace();
      (document.activeElement as HTMLElement | null)?.blur();
    };
    window.addEventListener('keydown', onShortcut, true);
    return () => window.removeEventListener('keydown', onShortcut, true);
  }, []);

  const refresh = useCallback(async (preserveFocus = false) => {
    try { useDeck.getState().setGames(await backend.listGames(), preserveFocus); }
    catch (err) { useDeck.getState().notify(`读取游戏库失败：${String(err)}`); }
  }, []);

  const backgroundScan=useCallback(async()=>{
    const state = useDeck.getState();
    if(backgroundScanning.current||state.busy||document.hidden||Date.now()-lastBackgroundScan.current<30*60*1000
      ||Object.values(state.sessions).some(session=>launchPending(session)||session.state==='running'))return;
    backgroundScanning.current=true;
    lastBackgroundScan.current=Date.now();
    clearTimeout(backgroundStatusTimer.current);
    // Discovery is silent unless it finds new games or encounters an error.
    let unsubscribe:(()=>void)|undefined;
    try{
      const report=await backend.scanGames(true);
      await refresh();
      localStorage.setItem('gamedeck-last-scan',String(Date.now()));
      if(report.inserted)useDeck.getState().notify(`已加入 ${report.inserted} 款新游戏`);
    }catch(error){setBackgroundStatus({label:`后台扫描失败：${String(error)}`,running:false});}
    finally{unsubscribe?.();backgroundScanning.current=false;backgroundStatusTimer.current=window.setTimeout(()=>setBackgroundStatus(null),12000);}
  },[refresh]);

  const scan = useCallback(async () => {
    if (useDeck.getState().busy || backgroundScanning.current) return;
    useDeck.getState().setBusy(true, '正在扫描游戏库…');
    try {
      const report = await backend.scanGames();
      await refresh();
      lastBackgroundScan.current=Date.now();localStorage.setItem('gamedeck-last-scan',String(Date.now()));
      useDeck.getState().notify(`扫描完成 · 发现 ${report.found} 款 · 新增 ${report.inserted} 款${report.errors.length ? ` · ${report.errors.length} 个警告` : ''}`);
    } catch (err) { useDeck.getState().notify(String(err)); }
    finally { useDeck.getState().setBusy(false); }
  }, [refresh]);

  const addManual = useCallback(async () => {
    try { await backend.addManual(); await refresh(); useDeck.getState().notify('导入完成，游戏已加入 Library'); }
    catch (err) { useDeck.getState().notify(String(err)); }
  }, [refresh]);

  const toggleFavorite = useCallback(async (game: Game) => {
    const value = !game.favorite;
    const store = useDeck.getState();
    store.updateGame(game.id, { favorite: value });
    try { await backend.setFavorite(game, value); store.notify(value ? '已加入收藏' : '已取消收藏'); }
    catch (err) { store.updateGame(game.id, { favorite: !value }); store.notify(`收藏失败：${String(err)}`); }
  }, []);

  const launch = useCallback(async (game: Game, accelerated = false) => {
    const current=useDeck.getState().sessions[game.id];
    if(launchPending(current)||current?.state==='running')return;
    useDeck.getState().acknowledgeGame(game.id);
    const accelerator=gameAccelerator(game.id);
    if(accelerator&&!accelerated){
      useDeck.getState().openAccelerators(accelerator,game.id);
      void backend.openAccelerator(accelerator).catch(error=>useDeck.getState().notify(String(error)));
      return;
    }
    const pending={session_id:crypto.randomUUID(),game_id:game.id,state:'preparing' as const,timestamp:new Date().toISOString()};
    useDeck.getState().beginSession(pending);
    try {
      const session = await backend.launch(game, pending.session_id, useInputDevice.getState().mode==='gamepad');
      if(useDeck.getState().setSession(session)&&session.state==='launcher_started'&&session.message)useDeck.getState().notify(session.message);
    } catch (err) { useDeck.getState().setSession({...pending,state:'failed',message:String(err)}); }

  }, []);

  useEffect(() => {
    let alive = true;
    let canScan=false;
    let initialScan:number|undefined;
    let titleRepair:number|undefined;
    let resumeScan:number|undefined;
    const periodicScan=window.setInterval(()=>{if(canScan)void backgroundScan();},30*60*1000);
    const onResume=()=>{
      clearTimeout(resumeScan);
      if(canScan&&document.visibilityState!=='hidden')resumeScan=window.setTimeout(()=>void backgroundScan(),1500);
    };
    window.addEventListener('focus',onResume);
    document.addEventListener('visibilitychange',onResume);
    void backend.startupLibrary().then(bundle=>{
      if(!alive)return;
      probeRef.current=bundle.probe;
      primeHeroPaths(bundle.hero_displays);
      useDeck.getState().setGames(bundle.games);
      if(bundle.navigation_probe)void import('./lib/navigationProbe').then(module=>module.runNavigationProbe());
      if(desktopMode&&!bundle.probe) {
        // Let the persisted library commit before discovery starts; never set global busy.
        canScan=true;
        titleRepair=window.setTimeout(()=>{void backend.repairTitles().then(games=>{if(alive)useDeck.getState().setGames(games);}).catch(()=>undefined);},8000);
        void backend.repairEpicArtwork().catch(()=>undefined);
        if(Date.now()-lastBackgroundScan.current>=30*60*1000)initialScan=window.setTimeout(()=>{if(alive)void backgroundScan();},5000);
      }
    }).catch(error=>{if(alive)setStartupError(String(error));});
    let unsub: (() => void) | undefined;
    void backend.subscribeSession(event => {
      if (!alive) return;
      const state = useDeck.getState();
      if (!state.setSession(event)) return;
      if (state.games.some(game=>game.id===event.game_id&&game.hidden) && !state.hiddenSpace) {
        if (event.state === 'exited') void refresh(true);
        return;
      }
      if (event.state === 'exited') {
        void refresh(true);
        useDeck.getState().notify('游戏已退出');
      }
    }).then(fn => { if (alive) unsub = fn; else fn(); });
    return () => { alive = false;canScan=false;unsub?.();clearTimeout(initialScan);clearTimeout(titleRepair);clearTimeout(resumeScan);clearInterval(periodicScan);window.removeEventListener('focus',onResume);document.removeEventListener('visibilitychange',onResume); };
  }, [refresh,backgroundScan]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => useDeck.getState().notify(''), 4600);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(()=>{let active=true;let unsub:(()=>void)|undefined;void backend.subscribeLibraryChanged(()=>{if(active)void refresh();}).then(fn=>{if(active)unsub=fn;else fn();});return()=>{active=false;unsub?.();};},[refresh]);

  const onAction = useCallback((action: InputAction) => {
    if(probeRef.current||!useDeck.getState().libraryLoaded)return;
    const control = new CustomEvent<InputAction>('deck-control', { detail: action, cancelable: true });
    if (!window.dispatchEvent(control)) return;
    const state = useDeck.getState();
    const switchPage = (next: Page) => state.setPage(next);
    if (state.page==='library'&&action==='BACK'&&document.activeElement?.closest('[data-library-controls]')){(document.activeElement as HTMLElement).blur();document.querySelector<HTMLElement>(`[data-game-id="${state.focusedId}"]`)?.focus();return;}
    if (action === 'BACK') {
      if(state.page==='library'&&state.hiddenSpace) {state.toggleHiddenSpace();return;}
      if(state.page==='accelerators'&&state.acceleratorGameId) state.setPage('details');
      else if (state.page === 'details') state.closeDetail();
      else if (state.page !== 'home') switchPage('home');
      else { const key = inputLabels(useInputDevice.getState()); state.notify(`${key.prev} / ${key.next}：游戏库 · ${key.menu}：设置`); }
      return;
    }
    if(state.page==='accelerators'){
      if(action==='PAGE_PREV'||action==='PAGE_NEXT'){const next=state.acceleratorProvider==='uu'?'xunyou':'uu';state.setAcceleratorProvider(next);if(document.activeElement?.closest('.accelerator-tabs'))document.getElementById(`accelerator-${next}`)?.focus();return;}
      const nodes=Array.from(document.querySelectorAll<HTMLElement>('.accelerators button:not(:disabled)')).filter(node=>node.getClientRects().length>0);
      const index=nodes.indexOf(document.activeElement as HTMLElement);
      if(action==='UP'||action==='LEFT')nodes[Math.max(0,index-1)]?.focus();
      if(action==='DOWN'||action==='RIGHT')nodes[Math.min(nodes.length-1,index+1)]?.focus();
      if(action==='CONFIRM')nodes[index<0?2:index]?.click();
      return;
    }
    if (state.page === 'settings') {
      if (action === 'PAGE_PREV' || action === 'PAGE_NEXT') { window.dispatchEvent(new CustomEvent('settings-tab', { detail: action === 'PAGE_NEXT' ? 1 : -1 })); return; }
      const nodes = Array.from(document.querySelectorAll<HTMLElement>('.settings [role=tabpanel] button:not(:disabled), .settings [role=tabpanel] input:not(:disabled), .settings [role=tabpanel] summary')).filter(node => node.getClientRects().length > 0);
      const i = nodes.indexOf(document.activeElement as HTMLElement);
      if (action === 'UP' || action === 'LEFT') nodes[Math.max(0, i < 0 ? 0 : i - 1)]?.focus();
      if (action === 'DOWN' || action === 'RIGHT') nodes[Math.min(nodes.length - 1, i + 1)]?.focus();
      if (action === 'CONFIRM' && i >= 0) { nodes[i].click(); if (nodes[i].tagName === 'INPUT') nodes[i].focus(); }
      return;
    }
    if (state.page === 'home') {
      if (action === 'PAGE_PREV' || action === 'PAGE_NEXT') return switchPage('library');
      if (action === 'MENU') return switchPage('settings');
      const games = homeGames(state.games,state.newGameIds);
      const index = Math.max(0, games.findIndex(g => g.id === state.focusedId));
      if (action === 'RIGHT' || action === 'DOWN') state.setFocus(games[Math.min(games.length - 1, index + 1)]?.id || null);
      if (action === 'LEFT' || action === 'UP') state.setFocus(games[Math.max(0, index - 1)]?.id || null);
      if (action === 'CONFIRM') { const id = state.focusedId ?? games[0]?.id; if (id) state.showDetail(id); }
      if (action === 'FAVORITE') { const g = state.games.find(g => g.id === state.focusedId); if (g) void toggleFavorite(g); }
      if (action === 'SEARCH') { switchPage('library'); requestAnimationFrame(() => document.getElementById('library-search')?.focus()); }
      return;
    }
    if (state.page === 'details') {
      const g = state.games.find(g => g.id === state.detailId);
      const nodes=Array.from(document.querySelectorAll<HTMLElement>('.details-actions button:not(:disabled),.detail-accelerator [role=combobox]'));
      const index=nodes.indexOf(document.activeElement as HTMLElement);
      if(action==='UP'||action==='LEFT'){nodes[Math.max(0,index-1)]?.focus();return;}
      if(action==='DOWN'||action==='RIGHT'){nodes[Math.min(nodes.length-1,index+1)]?.focus();return;}
      if(action==='MENU'&&g){state.openAccelerators(gameAccelerator(g.id)||'uu',g.id);return;}
      if (action === 'CONFIRM' && g) {if(index>=0)nodes[index].click();else void launch(g);}
      if (action === 'FAVORITE' && g) void toggleFavorite(g);
      if (action === 'PAGE_PREV' || action === 'PAGE_NEXT') state.closeDetail();
      return;
    }
    if (state.page === 'library') {
      const toolbar=document.activeElement?.closest('[data-library-controls]');
      if(toolbar&&action==='DOWN') { (document.activeElement as HTMLElement)?.blur(); document.querySelector<HTMLElement>(`[data-game-id="${state.focusedId}"]`)?.focus();return; }
      const filtered = visibleGames(state);
      const idx = Math.max(0, filtered.findIndex(g => g.id === state.focusedId));
      const cols = Number(document.getElementById('poster-wall')?.dataset.cols || 6);
      let next = idx;
      if (action === 'LEFT') next = Math.max(0, idx - 1);
      if (action === 'RIGHT') next = Math.min(filtered.length - 1, idx + 1);
      if (action === 'UP') { if(idx<cols){document.querySelector<HTMLElement>('.platform-tabs [aria-selected=true]')?.focus();return;} next = Math.max(0, idx - cols); }
      if (action === 'DOWN') next = Math.min(filtered.length - 1, idx + cols);
      if (['UP', 'DOWN', 'LEFT', 'RIGHT'].includes(action)) { if (filtered[next]) state.setFocus(filtered[next].id); return; }
      if (action === 'CONFIRM' && filtered[idx]) state.showDetail(filtered[idx].id);
      if (action === 'FAVORITE' && filtered[idx]) void toggleFavorite(filtered[idx]);
      if (action === 'MENU') {
        const sorts = ['recent', 'name', 'playtime'] as const;
        state.setSort(sorts[(sorts.indexOf(state.sort) + 1) % sorts.length]);
      }
      if (action === 'PAGE_PREV' || action === 'PAGE_NEXT') {
        const platforms = libraryPlatforms(spaceGames(state.games,state.hiddenSpace));
        const k = platforms.indexOf(state.platformFilter);
        const step = action === 'PAGE_NEXT' ? 1 : -1;
        state.setPlatformFilter(platforms[(k + step + platforms.length) % platforms.length]);
        const first = visibleGames(useDeck.getState())[0];
        state.setFocus(first?.id ?? null);
        if(toolbar) document.querySelector<HTMLElement>('.platform-tabs [aria-selected=true]')?.focus();
      }
      if (action === 'SEARCH') document.getElementById('library-search')?.focus();
    }
  }, [launch, toggleFavorite]);

  useDeckInput(onAction);
  return <div className={`app-shell ${accentEnabled ? '' : 'no-focus-motion'}`}>
    <Header />
    {!libraryLoaded && <main className="startup-shell"><strong>GameDeck</strong>{startupError?<><p>读取游戏库失败</p><button className="btn" onClick={()=>location.reload()}>重试</button></>:<LoaderCircle className="spin" size={24}/>}</main>}
    {libraryLoaded && (page==='home'||((page==='details'||page==='accelerators')&&previousPage==='home')) && <div className="home-route" style={{visibility:page==='home'?'visible':'hidden'}} inert={page!=='home'} aria-hidden={page!=='home'}><Home active={page==='home'} onLaunch={launch} onFavorite={toggleFavorite}/></div>}
    {libraryLoaded && (page==='library'||((page==='details'||page==='accelerators')&&previousPage==='library')) && <div className="library-route" style={{visibility:page==='library'?'visible':'hidden'}} inert={page!=='library'} aria-hidden={page!=='library'}><Suspense fallback={<main className="startup-shell"><LoaderCircle className="spin"/></main>}><Library key={hiddenSpace?'hidden':'public'} active={page==='library'}/></Suspense></div>}
    {libraryLoaded && page === 'details' && <Suspense fallback={<main className="startup-shell"><LoaderCircle className="spin"/></main>}><Details onLaunch={launch} onFavorite={toggleFavorite}/></Suspense>}
    {libraryLoaded && page === 'accelerators' && <Suspense fallback={<main className="startup-shell"><LoaderCircle className="spin"/></main>}><Accelerators onContinue={game=>{useDeck.getState().setPage('details');void launch(game,true);}}/></Suspense>}
    {libraryLoaded && page === 'settings' && <Suspense fallback={<main className="startup-shell"><LoaderCircle className="spin"/></main>}><Settings onScan={() => void scan()} onImport={() => void addManual()}/></Suspense>}
    {(busy||backgroundStatus)&&<div className="task-status-stack" aria-live="polite">
      {busy&&<div className="busy-indicator" role="status"><LoaderCircle size={15} className="spin"/>{busyLabel}</div>}
      {backgroundStatus&&<div className="busy-indicator background-scan-status" role="status" title={backgroundStatus.label}>{backgroundStatus.running?<LoaderCircle size={15} className="spin"/>:<CheckCircle2 size={15}/>}<span>{backgroundStatus.label}</span></div>}
    </div>}
    {toast && <div className="toast" role="status"><span className="toast-icon">{toast.includes('失败') || toast.includes('Error') ? <AlertCircle size={18} /> : <CheckCircle2 size={18}/>}</span><span>{toast}</span><button onClick={() => useDeck.getState().notify('')} aria-label="关闭提示"><X size={16}/></button></div>}
    <div className="global-glow" aria-hidden="true" />
  </div>;
}
