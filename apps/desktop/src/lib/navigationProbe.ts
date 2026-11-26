// Runs only in an explicitly enabled hidden native probe; no launch or artwork operations.
import {invoke} from '@tauri-apps/api/core';
import {setInputDevice} from './inputDevice';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {syncWindowState} from './window';
import {refreshDisplay,setInterfaceScale,useDisplayState} from './display';
import {backend} from './backend';
import {useDeck} from './store';
import {prepareScene} from './artwork';
const pause=(ms=80)=>new Promise(resolve=>setTimeout(resolve,ms));
async function waitFor(selector:string){for(let i=0;i<60;i++){const node=document.querySelector<HTMLElement>(selector);if(node)return node;await pause();}throw new Error(`Timed out: ${selector}`);}
export async function runNavigationProbe(){
 const checks:string[]=[];const status=backend.refreshGameStatus;backend.refreshGameStatus=async()=>null;
 const diagnostics:Record<string,unknown>={pixelRatio:devicePixelRatio,viewport:[innerWidth,innerHeight]};
 const longTasks:number[]=[];
 const observer=typeof PerformanceObserver!=='undefined'?new PerformanceObserver(list=>{for(const entry of list.getEntries())longTasks.push(entry.duration);}):null;
 if(observer&&PerformanceObserver.supportedEntryTypes.includes('longtask'))observer.observe({type:'longtask',buffered:true});
 const gl=document.createElement('canvas').getContext('webgl');
 if(gl){const debug=gl.getExtension('WEBGL_debug_renderer_info');diagnostics.renderer=debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):gl.getParameter(gl.RENDERER);gl.getExtension('WEBGL_lose_context')?.loseContext();}
 const check=(name:string,condition:boolean)=>{if(!condition)throw new Error(name);checks.push(name);};
 try{
  useDeck.getState().setPage('library');
  const scroll=await waitFor('.library-scroll');await pause(200);
  scroll.scrollTop=Math.min(600,scroll.scrollHeight-scroll.clientHeight);await pause(160);
  check('fullscreen hides native window controls',!document.querySelector('.window-controls')&&!!document.querySelector('.header-fullscreen'));
  check('library hides OS scrollbar while preserving scrolling',getComputedStyle(scroll).scrollbarWidth==='none'&&getComputedStyle(scroll).overflowY==='auto');
  const wall=document.querySelector('#poster-wall');
  const bounds=scroll.getBoundingClientRect();
  const card=Array.from(scroll.querySelectorAll<HTMLElement>('[data-game-id]')).find(node=>{const b=node.getBoundingClientRect();return b.top>=bounds.top&&b.bottom<=bounds.bottom;})||scroll.querySelector<HTMLElement>('[data-game-id]');
  if(!card?.dataset.gameId)throw new Error('No public game card');
  const id=card.dataset.gameId;useDeck.getState().setFocus(id,'pointer');await pause();
  const offset=scroll.scrollTop;const image=card.querySelector('img');
  useDeck.getState().showDetail(id);await waitFor('.details');await pause();
  check('library DOM retained beneath detail',document.querySelector('.library-scroll')===scroll&&document.querySelector('#poster-wall')===wall);
  check('library is inert beneath detail',!!document.querySelector('.library-route[inert]'));
  useDeck.getState().closeDetail();await pause(200);
  check('return reuses library DOM',document.querySelector('.library-scroll')===scroll);
  check('return preserves scroll offset',Math.abs(scroll.scrollTop-offset)<2);
  check('return preserves card image element',card.querySelector('img')===image&&document.contains(card));
  const before=useDeck.getState().games;
  useDeck.getState().setGames(before.map(game=>({...game,update_checked_at:new Date().toISOString()})));await pause();
  check('unchanged background results preserve game array',useDeck.getState().games===before);
  check('unchanged result preserves card image',card.querySelector('img')===image);
  check('background refresh does not set global busy',!useDeck.getState().busy);
  const logoGame=useDeck.getState().games.find(game=>!game.hidden&&game.logo&&!game.hero_source?.startsWith('background-promo-branded-'));
  if(logoGame){useDeck.getState().setPage('home');useDeck.getState().setFocus(logoGame.id);const title=await waitFor('.game-title-art img') as HTMLImageElement;await title.decode();check('original transparent title logo decodes',title.naturalWidth>0);const home=document.querySelector('.home');const backdrop=document.querySelector('.hero-image-layer');useDeck.getState().showDetail(logoGame.id);await waitFor('.details');useDeck.getState().closeDetail();await pause();check('home survives detail return',document.querySelector('.home')===home&&document.querySelector('.hero-image-layer')===backdrop);}
  useDeck.getState().setPage('home');
  const switches=useDeck.getState().games.filter(game=>!game.hidden&&game.hero).slice(0,3);
  for(const game of switches)await prepareScene(game);
  for(const game of [...switches,...switches.slice().reverse()]){
    const expected=await prepareScene(game);const start=performance.now();useDeck.getState().setFocus(game.id);
    for(let i=0;i<30&&document.querySelector<HTMLElement>('.home')?.dataset.featuredId!==game.id;i++)await pause(20);
    check(`scene committed together: ${game.original_name}`,document.querySelector<HTMLElement>('.home')?.dataset.featuredId===game.id);
    check(`correct backdrop: ${game.original_name}`,(document.querySelector('.hero-image-current') as HTMLImageElement)?.src===expected.background);
    const title=document.querySelector('.hero-title');const logo=title?.querySelector('img');
    check(`correct title: ${game.original_name}`,expected.logo?logo?.getAttribute('alt')===game.name:title?.textContent===game.name);
    check(`warm scene under 600ms: ${game.original_name}`,performance.now()-start<600);
  }
  if(switches.length>1){useDeck.getState().setFocus(switches[0].id);useDeck.getState().setFocus(switches[1].id);await pause(200);check('rapid switching keeps latest selection',document.querySelector<HTMLElement>('.home')?.dataset.featuredId===switches[1].id);}
  const launchGame=switches[0];
  if(launchGame){
    useDeck.getState().setFocus(launchGame.id);await pause(200);
    const event={session_id:'button-probe',game_id:launchGame.id,timestamp:new Date().toISOString()};
    useDeck.getState().beginSession({...event,state:'preparing'});
    useDeck.getState().setSession({...event,state:'awaiting_game'});await pause();
    check('launch spinner is on primary button',!!document.querySelector('.hero-actions .launch-control button[aria-busy=true] .spin')&&document.querySelector('.hero-actions .launch-control')?.textContent?.includes('启动中')===true);
    check('launch does not create top-right busy indicator',!useDeck.getState().busy&&!document.querySelector('.busy-indicator'));
    useDeck.getState().setSession({...event,state:'running'});await pause();
    check('confirmed game switches button to running',document.querySelector('.hero-actions .launch-control')?.textContent?.includes('运行中')===true&&!document.querySelector('.hero-actions .launch-control .spin'));
    useDeck.getState().setSession({...event,state:'awaiting_game'});await pause();
    check('late handoff cannot replace running button',document.querySelector('.hero-actions .launch-control')?.getAttribute('data-launch-state')==='running');
    useDeck.getState().showDetail(launchGame.id);await waitFor('.details');await pause();
    check('detail shares the same running state',document.querySelector('.details-actions .launch-control')?.textContent?.includes('运行中')===true);
    useDeck.getState().setSession({...event,state:'exited'});await pause();
    check('exited game restores launch button',document.querySelector('.details-actions .launch-control button')?.hasAttribute('disabled')===false);
    const retry={...event,session_id:'button-probe-retry'};
    useDeck.getState().beginSession({...retry,state:'preparing'});
    useDeck.getState().setSession({...retry,state:'running'});
    useDeck.getState().setSession({...event,state:'exited'});await pause();
    check('late exit cannot end the newer running session',document.querySelector('.details-actions .launch-control')?.getAttribute('data-launch-state')==='running');
    useDeck.getState().setSession({...retry,state:'failed',message:'测试失败信息'});await pause();
    check('failure is shown beside retry button',document.querySelector('.details-actions .launch-error')?.textContent==='测试失败信息');
  }
  useDeck.getState().setPage('settings');await waitFor('.epic-account');await pause(100);
  check('settings exposes Epic account library connection',document.querySelector('.epic-account h3')?.textContent==='Epic 账号'&&!!document.querySelector('.epic-account button'));
  window.dispatchEvent(new CustomEvent('settings-tab',{detail:1}));await waitFor('.epic-artwork');
  check('Epic artwork completion is available independently of account login',document.querySelector('.epic-artwork')?.textContent?.includes('补全缺失素材')===true);
  window.dispatchEvent(new CustomEvent('settings-tab',{detail:1}));await waitFor('[aria-label="界面缩放"]');
  await refreshDisplay();
  check('display settings exposes monitor scale and game following',useDisplayState.getState().monitors.length>0&&!!document.querySelector('[aria-label="大厅显示器"]')&&!!document.querySelector('[aria-label="游戏跟随大厅屏幕"]'));
  const oldWidth=window.innerWidth;const oldZoom=useDisplayState.getState().zoom;
  await setInterfaceScale('0.9');await pause(180);
  check('native interface scaling changes the renderer viewport',Math.abs(window.innerWidth-oldWidth*oldZoom/.9)<4);
  await setInterfaceScale('auto');
  await getCurrentWindow().setFullscreen(false);await syncWindowState();await waitFor('.window-controls');
  check('windowed mode restores window controls',!!document.querySelector('.window-controls')&&!document.querySelector('.header-fullscreen'));
  await getCurrentWindow().setFullscreen(true);await syncWindowState();await pause(150);
  check('returning to fullscreen hides window controls again',!document.querySelector('.window-controls')&&!!document.querySelector('.header-fullscreen'));
  const scale = await waitFor('[aria-label="界面缩放"]');
  const pressF11 = (target: EventTarget, repeat = false) => target.dispatchEvent(new KeyboardEvent('keydown',{key:'F11',bubbles:true,cancelable:true,repeat}));
  scale.focus();
  pressF11(scale);
  const controls = await waitFor('.window-controls');
  check('F11 exits native fullscreen from a settings control',!await getCurrentWindow().isFullscreen());
  check('F11 restores enabled minimize, maximize and close buttons', ['最小化','最大化','关闭窗口'].every(label=>!!controls.querySelector(`button[aria-label="${label}"]:not(:disabled)`)));
  pressF11(controls,true);await pause(250);
  check('held F11 leaves the window in the new mode',!await getCurrentWindow().isFullscreen()&&document.contains(controls));
  pressF11(controls);
  for(let i=0;i<40&&document.contains(controls);i++)await pause(20);
  check('F11 restores native fullscreen from the window controls',await getCurrentWindow().isFullscreen()&&!document.contains(controls));
  useDeck.getState().setPage('home');await waitFor('.home');await pause();
  setInputDevice({mode:'gamepad',family:'xbox'});await pause();
  const primary=await waitFor('.hero-actions button');
  check('controller mode hides the cursor over clickable controls',getComputedStyle(primary).cursor==='none');
  const selected=useDeck.getState().focusedId;const other=Array.from(document.querySelectorAll<HTMLElement>('.home [data-game-id]')).find(node=>node.dataset.gameId!==selected);
  other?.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,pointerType:'mouse',movementX:0,movementY:0}));await pause();
  check('stationary cursor cannot steal controller selection',useDeck.getState().focusedId===selected&&document.documentElement.dataset.controllerCursor==='hidden');
  window.dispatchEvent(new PointerEvent('pointermove',{pointerType:'mouse',movementX:5,movementY:0}));await pause();
  check('deliberate mouse movement restores the normal pointer',getComputedStyle(primary).cursor!=='none'&&!document.documentElement.dataset.controllerCursor);
  const native=await invoke<{available:boolean,pads:unknown[]}>('native_gamepad_status');
  check('native controller backend loads without mouse activation',native.available&&Array.isArray(native.pads));
  diagnostics.longTaskCount=longTasks.length;diagnostics.longestTaskMs=Math.max(0,...longTasks);
  await invoke('startup_probe_result',{passed:true,checks,error:null,diagnostics});
 }catch(error){await invoke('startup_probe_result',{passed:false,checks,error:String(error),diagnostics});}
 finally{observer?.disconnect();backend.refreshGameStatus=status;}
}
