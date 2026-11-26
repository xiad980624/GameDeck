import {availableMonitors,currentMonitor,getCurrentWindow,type Monitor} from '@tauri-apps/api/window';
import {getCurrentWebview} from '@tauri-apps/api/webview';
import {isTauri} from '@tauri-apps/api/core';
import {create} from 'zustand';

export const scaleOptions=['auto','0.8','0.9','1','1.1','1.2'] as const;
export type InterfaceScale=typeof scaleOptions[number];
const stored=localStorage.getItem('gamedeck-ui-scale');
export const useDisplayState=create(()=>({monitors:[] as Monitor[],current:'',uiScale:(scaleOptions.includes(stored as InterfaceScale)?stored:'auto') as InterfaceScale,followGame:localStorage.getItem('gamedeck-follow-display')!=='off',zoom:1}));
export const monitorId=(monitor:Monitor)=>`${monitor.name??''}|${monitor.position.x}|${monitor.position.y}`;

export function automaticZoom(logicalHeight:number,fullscreen:boolean){return fullscreen?Math.max(.8,Math.min(1,logicalHeight/1080)):1;}
let lastZoom:number|undefined;
export async function refreshDisplay(){
 if(!isTauri())return;
 const win=getCurrentWindow();
 const [monitors,monitor,size,scale,fullscreen]=await Promise.all([availableMonitors(),currentMonitor(),win.innerSize(),win.scaleFactor(),win.isFullscreen()]);
 useDisplayState.setState({monitors,current:monitor?monitorId(monitor):''});
 const choice=useDisplayState.getState().uiScale;
 const zoom=choice==='auto'?automaticZoom(size.height/scale,fullscreen):Number(choice);
 if(lastZoom===undefined||Math.abs(lastZoom-zoom)>.005){await getCurrentWebview().setZoom(zoom);lastZoom=zoom;useDisplayState.setState({zoom});}
}
export async function setInterfaceScale(value:string){
 if(!scaleOptions.includes(value as InterfaceScale))return;
 const previous=useDisplayState.getState().uiScale;
 useDisplayState.setState({uiScale:value as InterfaceScale});
 try{await refreshDisplay();localStorage.setItem('gamedeck-ui-scale',value);}
 catch(error){useDisplayState.setState({uiScale:previous});throw error;}
}
export function setFollowGame(value:boolean){localStorage.setItem('gamedeck-follow-display',value?'on':'off');useDisplayState.setState({followGame:value});}
export async function selectDisplay(id:string){
 const monitor=useDisplayState.getState().monitors.find(m=>monitorId(m)===id);if(!monitor)return;
 await getCurrentWindow().setFullscreenOnMonitor(monitor.position);
 localStorage.setItem('gamedeck-display',monitorId(monitor));
 await refreshDisplay();
}
export async function initializeDisplay(){
 if(!isTauri())return()=>{};
 const win=getCurrentWindow();let timer:ReturnType<typeof setTimeout>|undefined;let disposed=false;
 await refreshDisplay();
 // The lobby stays on whichever monitor Windows opens it on (DISPLAY1 and the
 // TV are a duplicate pair, so they show the same desktop). Do not restore a
 // saved monitor here — that could drag the lobby off the TV on startup.
 const changed=()=>{clearTimeout(timer);timer=setTimeout(()=>{if(!disposed)void refreshDisplay().catch(()=>undefined);},120);};
 const unlisten=await Promise.all([win.onMoved(changed),win.onResized(changed),win.onScaleChanged(changed)]);
 return()=>{disposed=true;clearTimeout(timer);unlisten.forEach(fn=>fn());};
}
