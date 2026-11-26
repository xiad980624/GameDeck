// Development-only UI integration probe. Not referenced by the production entry.
import { createRoot } from 'react-dom/client';
import App from './App';
import { useDeck } from './lib/store';
import { useInputDevice } from './lib/inputDevice';
import { desktopMode } from './lib/backend';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { backend } from './lib/backend';
import { gameAccelerator, saveGameAccelerator } from './lib/acceleration';
import './styles/index.css';

const result = document.createElement('pre');
result.id = 'smoke-result';
Object.assign(result.style, { position:'fixed',left:'12px',bottom:'12px',zIndex:'2000',maxHeight:'40vh',overflow:'auto',padding:'20px',background:'#10152af2',color:'#c8eecf',fontSize:'14px',border:'1px solid #657c85',pointerEvents:'none' });
document.body.append(result);
result.textContent = 'RUNNING · rendered UI + Gamepad API';
let controller: Gamepad | null = null;
const physicalControllers = Array.from(navigator.getGamepads?.()||[]).filter(Boolean).length;
Object.defineProperty(navigator,'getGamepads',{ configurable:true,value:()=>[controller] });
createRoot(document.getElementById('root')!).render(<App/>);
const pause = (ms=100) => new Promise(resolve=>setTimeout(resolve,ms));
const lines: string[] = [];
function check(name:string, condition:boolean) { lines.push(`${condition?'PASS':'FAIL'} ${name}`); result.textContent = lines.join('\n'); if(!condition)throw new Error(name); }
function makePad(id='Xbox Wireless Controller',pressed=-1) {
  return { id,connected:true,axes:[0,0],buttons:Array.from({length:16},(_,i)=>({pressed:i===pressed,touched:i===pressed,value:i===pressed?1:0})),mapping:'standard',index:0,timestamp:performance.now() } as unknown as Gamepad;
}
async function pulse(button:number,id='Xbox Wireless Controller') { controller=makePad(id,button);await pause();controller=makePad(id);await pause(); }
function key(key:string) { window.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true})); }
async function verifyImage(path:string|null|undefined,label:string){
  const source=convertFileSrc(path||'');const image=new Image();image.src=source;
  try{await image.decode();}catch{
    let diagnostic='';try{const response=await fetch(source);diagnostic=`HTTP ${response.status}, ${response.headers.get('content-type')}, ${(await response.arrayBuffer()).byteLength} bytes`;}catch(error){diagnostic=String(error);}
    throw new Error(`${label}: ${path} | ${diagnostic}`);
  }
  return image;
}
async function run() {
  await fetch('/__input-probe',{method:'POST',body:JSON.stringify({running:true,desktopMode,stage:'waiting-library'})});
  await pause(400);
  for (let i=0;i<2400&&(useDeck.getState().busy||(desktopMode&&useDeck.getState().demo));i++) await pause(50);
  if(desktopMode&&useDeck.getState().demo)throw new Error('Native library did not load: '+useDeck.getState().toast);
  controller=makePad();await pause();check('idle controller keeps keyboard hints',useInputDevice.getState().mode==='keyboard');
  const before=useDeck.getState().focusedId;await pulse(15);check('Xbox D-pad selects next real card',useDeck.getState().focusedId!==before);
  check('Xbox changes rendered global hints',!!document.querySelector('[aria-label="手柄操作"]')&&document.querySelector('.input-hints')!.textContent!.includes('LB'));
  await pulse(0);check('A opens detail page',useDeck.getState().page==='details');
  await pulse(1);check('B returns to home',useDeck.getState().page==='home');
  await pulse(3);check('Y opens settings',useDeck.getState().page==='settings');
  await pulse(5);check('RB switches settings tab',document.querySelector('[role=tab][aria-selected=true]')?.textContent==='素材与刮削');
  (document.querySelector('[role=combobox][aria-label="匹配来源"]') as HTMLElement).focus();await pulse(0);check('A opens styled listbox',!!document.querySelector('[role=listbox]'));
  await pulse(13);await pulse(0);check('D-pad and A select option',document.querySelector('[aria-label="匹配来源"]')?.textContent==='SteamGridDB'&&!document.querySelector('[role=listbox]'));
  await pulse(4);check('LB switches back without leaving settings',useDeck.getState().page==='settings'&&document.querySelector('[role=tab][aria-selected=true]')?.textContent==='游戏库与账号');
  await pulse(5,'Wireless Controller Vendor:054c');check('Sony changes rendered hints to symbols',useInputDevice.getState().family==='playstation'&&document.querySelector('.input-hints')!.textContent!.includes('×')&&document.querySelector('.input-hints')!.textContent!.includes('L1'));
  controller=null;await pause();check('disconnect restores keyboard hints',useInputDevice.getState().mode==='keyboard');
  key('Escape');await pause();key('e');await pause();check('keyboard switches to library',useDeck.getState().page==='library');
  const sort=document.querySelector('[aria-label="游戏排序"]') as HTMLElement;sort.focus();sort.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true}));await pause();
  sort.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowDown',bubbles:true,cancelable:true}));await pause();sort.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true}));await pause();
  check('keyboard chooses styled library sort',useDeck.getState().sort==='name'&&!document.querySelector('[role=listbox]'));
  if (desktopMode) {
    for (const game of useDeck.getState().games.filter(g=>g.platform==='epic')) {
      const image=await verifyImage(game.poster,`Epic cover ${game.name}`); check(`Epic cover decodes: ${game.name}`,image.naturalWidth>0);
    }
    if (import.meta.env.VITE_EPIC_LAUNCH_PROBE==='true') {
      const epic=useDeck.getState().games.find(g=>g.platform==='epic'&&g.external_id==='Ovenbird');
      if (!epic) throw new Error('Epic launch probe game not found');
      const launched=await backend.launch(epic,crypto.randomUUID());
      check('Epic native launch command uses correct provider',launched.state==='launcher_started'&&!!launched.message?.includes('Epic'));
    }
  }
  check('platform tabs replace all-platform dropdown',!document.querySelector('[role=combobox][aria-label="游戏平台"]')&&document.querySelectorAll('.platform-tabs [role=tab]').length>=4);
  check('library defaults to Steam',useDeck.getState().platformFilter==='steam');
  (document.activeElement as HTMLElement)?.blur();
  await pulse(5);check('RB switches to Epic platform',useDeck.getState().platformFilter==='epic');
  await pulse(5);check('RB switches to Battle.net platform',useDeck.getState().platformFilter==='battlenet');
  await pulse(4);check('LB returns to Epic platform',useDeck.getState().platformFilter==='epic');
  const battleTab=document.querySelector('#platform-battlenet') as HTMLElement;
  battleTab.focus();await pulse(0);
  check('A selects focused platform tab',useDeck.getState().platformFilter==='battlenet'&&battleTab.getAttribute('aria-selected')==='true');
  useDeck.getState().setBusy(true,'正在扫描游戏库…');await pause();
  check('busy indicator identifies scan operation',document.querySelector('.busy-indicator')?.textContent?.includes('正在扫描游戏库')===true);
  useDeck.getState().setBusy(false);await pause();
  check('completed operation clears indicator',!document.querySelector('.busy-indicator'));
  const tabKey=(value:string)=>document.activeElement?.dispatchEvent(new KeyboardEvent('keydown',{key:value,bubbles:true,cancelable:true}));
  tabKey('ArrowLeft');await pause();check('keyboard arrow selects previous tab',useDeck.getState().platformFilter==='epic'&&document.activeElement?.id==='platform-epic');
  tabKey('e');await pause();check('keyboard E selects next tab while toolbar focused',useDeck.getState().platformFilter==='battlenet'&&document.activeElement?.id==='platform-battlenet');
  tabKey('End');await pause();check('keyboard End selects last platform',document.activeElement?.getAttribute('aria-selected')==='true'&&useDeck.getState().platformFilter!=='battlenet');
  battleTab.click();battleTab.focus();await pause();
  const shown=Array.from(document.querySelectorAll<HTMLElement>('[data-game-id]')).map(node=>node.dataset.gameId);
  const battle=useDeck.getState().games.filter(g=>g.platform==='battlenet'&&g.installed);
  check('platform wall excludes other clients',shown.length>0&&shown.every(id=>battle.some(game=>game.id===id)));
  if(desktopMode){
    check('Battle.net discovers real installs',battle.length===3);
    check('Battle.net distinguishes regional editions',battle.some(g=>g.name.includes('国服'))&&battle.some(g=>g.name.includes('国际服')));
    for(const game of battle){
      const cover=await verifyImage(game.poster,`Battle.net cover ${game.name}`);
      check(`Battle.net uses product cover: ${game.name}`,!!game.poster?.includes('-battlenet-v2-')&&(game.external_id==='diablo_iv'?cover.naturalWidth===1920:cover.naturalHeight>cover.naturalWidth));

      const image=await verifyImage(game.hero,`Battle.net background ${game.name}`);
      check(`Battle.net background decodes: ${game.name}`,image.naturalWidth>=1000);
      check(`Battle.net reports version without fake latest: ${game.name}`,!!game.installed_version&&game.update_status==='unknown');
    }
    check('Steam update state is read',useDeck.getState().games.some(g=>g.platform==='steam'&&g.installed&&g.update_status!=='unknown'));
  }

  const navButtons=Array.from(document.querySelectorAll<HTMLButtonElement>('.primary-nav button'));
  check('search navigation removed',navButtons.every(button=>button.textContent?.trim()!=='搜索'));
  navButtons.find(button=>button.textContent?.trim()==='加速器')?.click();await pause(200);
  check('accelerator navigation opens page',useDeck.getState().page==='accelerators'&&!!document.querySelector('.accelerator-tabs'));
  await pulse(5);check('RB switches to Xunyou accelerator',useDeck.getState().acceleratorProvider==='xunyou');
  await pulse(4);check('LB switches to UU accelerator',useDeck.getState().acceleratorProvider==='uu');
  if(desktopMode){
    const clients=await backend.accelerators();
    check('UU discovery finds actual E drive client',clients.some(client=>client.id==='uu'&&!!client.path?.toLowerCase().includes('e:')&&client.path.toLowerCase().endsWith('uu_launcher.exe')));
    check('Xunyou discovery finds actual client',clients.some(client=>client.id==='xunyou'&&!!client.path?.toLowerCase().endsWith('xunyou.exe')));
    if(import.meta.env.VITE_ACCELERATOR_LAUNCH_PROBE==='fixture'){
      try{
        await invoke('set_accelerator_path',{provider:'uu',path:'G:\\游戏大厅\\gamedeck\\release\\validation-data\\AcceleratorOpenProbe.exe'});
        await backend.openAccelerator('uu');check('Windows shell launches configured GUI executable',true);
      }finally{await backend.resetAccelerator('uu');}
    }
    if(import.meta.env.VITE_ACCELERATOR_LAUNCH_PROBE==='true'){
      await backend.openAccelerator('uu');check('UU native open command succeeds',true);
      await backend.openAccelerator('xunyou');check('Xunyou native open command succeeds',true);
    }
  }
  const game=useDeck.getState().games.find(game=>game.installed)!;
  const originalPreference=gameAccelerator(game.id);const originalLaunch=backend.launch;const originalOpen=backend.openAccelerator;
  let launchCalls=0;let acceleratorCalls=0;
  try{
    backend.launch=async()=>{launchCalls++;return {session_id:'probe',game_id:game.id,state:'launcher_started',timestamp:new Date().toISOString()};};
    backend.openAccelerator=async(provider)=>{if(provider==='uu')acceleratorCalls++;};
    saveGameAccelerator(game.id,'uu');useDeck.getState().setPage('library');useDeck.getState().showDetail(game.id);await pause();
    (document.querySelector('.details-actions .btn-primary') as HTMLButtonElement).click();await pause(200);
    check('game preference gates launch behind accelerator page',useDeck.getState().page==='accelerators'&&useDeck.getState().acceleratorGameId===game.id&&acceleratorCalls===1&&launchCalls===0);
    (document.querySelector('.accelerator-continue .btn-primary') as HTMLButtonElement).click();await pause(200);
    check('continue launches game once without reopening accelerator',useDeck.getState().page==='details'&&launchCalls===1&&acceleratorCalls===1);
    await pulse(1);check('return after acceleration preserves library origin',useDeck.getState().page==='library');
  }finally{backend.launch=originalLaunch;backend.openAccelerator=originalOpen;saveGameAccelerator(game.id,originalPreference);}

  const originalGames=useDeck.getState().games;
  const originalNew=useDeck.getState().newGameIds;
  const knownBefore=localStorage.getItem('gamedeck-known-games');const pendingBefore=localStorage.getItem('gamedeck-new-games');
  try{
    useDeck.getState().setPage('home');
    const added={...originalGames[0],id:'ui-new-game-probe',name:'新增入库验证',platform:'manual',external_id:'probe',added_at:new Date().toISOString(),last_played_at:null};
    useDeck.getState().setGames([...originalGames,added]);await pause(180);
    check('new game is first on home wall',document.querySelector('.home-strip [data-game-id]')?.getAttribute('data-game-id')===added.id);
    check('new game is focused and has badge',useDeck.getState().focusedId===added.id&&!!document.querySelector(`[data-game-id="${added.id}"] [aria-label="新增游戏"]`));
    await pulse(15);check('controller order follows home display order',useDeck.getState().focusedId===document.querySelectorAll('.home-strip [data-game-id]')[1]?.getAttribute('data-game-id'));
    useDeck.getState().showDetail(added.id);await pause();check('viewing game clears new marker',!useDeck.getState().newGameIds.includes(added.id));
  }finally{
    useDeck.setState({games:originalGames,newGameIds:originalNew,page:'library',detailId:null});
    for(const [key,value] of [['gamedeck-known-games',knownBefore],['gamedeck-new-games',pendingBefore]]){if(value===null)localStorage.removeItem(key!);else localStorage.setItem(key!,value!);}
  }
  if(desktopMode){
    check('background reports real source resolution',originalGames.some(game=>!!game.hero_width&&!!game.hero_height&&!!game.hero_source));
    const steam=originalGames.find(game=>game.platform==='steam'&&game.external_id==='570')||originalGames.find(game=>game.platform==='steam');
    if(steam&&import.meta.env.VITE_BACKGROUND_ONLINE_PROBE==='true'){
      const old=steam.hero;let online:{success:boolean;error?:string;width?:number|null;height?:number|null};
      try{
        const refreshed=await backend.improveBackground(steam.id);const upgraded=refreshed.find(game=>game.id===steam.id)!;
        const image=await verifyImage(upgraded.hero,'upgraded Steam background');
        check('downloaded original background decodes',image.naturalWidth>0);
        online={success:true,width:upgraded.hero_width,height:upgraded.hero_height};
      }catch(error){
        const refreshed=await backend.listGames();check('failed online upgrade preserves existing background',refreshed.find(game=>game.id===steam.id)?.hero===old);
        online={success:false,error:String(error)};
      }
      (window as unknown as {backgroundProbe:unknown}).backgroundProbe=online;
    }
  }
  result.textContent=`ALL ${lines.length} UI CHECKS PASSED\n${lines.join('\n')}`;
  document.title=`PASS ${lines.length} · GameDeck 输入实测`;
  await fetch('/__input-probe',{method:'POST',body:JSON.stringify({passed:true,checks:lines,mode:'virtual-gamepad-rendered-ui',physicalControllers,games:useDeck.getState().games.length,backgroundProbe:(window as unknown as {backgroundProbe?:unknown}).backgroundProbe})});
  if (desktopMode) {
    // Keep the parent alive while observing an external launcher's startup.
    if (import.meta.env.VITE_EPIC_LAUNCH_PROBE==='true') await pause(45000);
    await getCurrentWindow().close();
  }
}
void run().catch(error=>{result.style.color='#ff9a9a';result.textContent+='\nSTOPPED: '+String(error);document.title='FAIL · GameDeck 输入实测';void fetch('/__input-probe',{method:'POST',body:JSON.stringify({passed:false,checks:lines,error:String(error),toast:useDeck.getState().toast,games:useDeck.getState().games.length,demo:useDeck.getState().demo})}).then(()=>{if(desktopMode)void getCurrentWindow().close();});});
