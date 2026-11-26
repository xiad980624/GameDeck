import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));const ts=require('typescript');
function setup(saved={}){
 const storage=new Map(Object.entries(saved));let state;let monitor;const zooms=[],moves=[];
 const monitors=[{name:'PC',position:{x:0,y:0}},{name:'TV',position:{x:-3840,y:0}}];monitor=monitors[0];
 const win={innerSize:async()=>({height:2160}),scaleFactor:async()=>3,isFullscreen:async()=>true,setFullscreenOnMonitor:async position=>{moves.push(position);monitor=monitors.find(m=>m.position===position);},onMoved:async()=>()=>{},onResized:async()=>()=>{},onScaleChanged:async()=>()=>{}};
 const modules={'@tauri-apps/api/core':{isTauri:()=>true},'@tauri-apps/api/window':{getCurrentWindow:()=>win,availableMonitors:async()=>monitors,currentMonitor:async()=>monitor},'@tauri-apps/api/webview':{getCurrentWebview:()=>({setZoom:async value=>zooms.push(value)})},zustand:{create:init=>{state=init();return {getState:()=>state,setState:patch=>{state={...state,...patch};}};}}};
 const exports={};vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/display.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports,require:name=>modules[name],localStorage:{getItem:key=>storage.get(key)??null,setItem:(key,value)=>storage.set(key,value)},setTimeout,clearTimeout});return {api:exports,storage,zooms,moves,state:()=>state,monitors};
}
test('automatic UI scale compensates high DPI TVs and keeps a readable lower bound',()=>{
 const {api}=setup();assert.equal(api.automaticZoom(720,true),.8);assert.equal(api.automaticZoom(1080,true),1);assert.equal(api.automaticZoom(2160,true),1);assert.equal(api.automaticZoom(600,false),1);
});
test('changing interface scale updates the native webview and survives reinitialization',async()=>{
 const f=setup();await f.api.refreshDisplay();assert.equal(f.zooms[0],.8);await f.api.setInterfaceScale('0.9');assert.equal(f.zooms.at(-1),.9);assert.equal(f.storage.get('gamedeck-ui-scale'),'0.9');await f.api.refreshDisplay();assert.equal(f.zooms.length,2);
});
test('startup restores a connected selected display and ignores an unplugged one',async()=>{
 const f=setup({'gamedeck-display':'TV|-3840|0'});const cleanup=await f.api.initializeDisplay();assert.equal(f.moves.length,1);assert.equal(f.state().current,'TV|-3840|0');cleanup();
 const disconnected=setup({'gamedeck-display':'missing|9999|0'});(await disconnected.api.initializeDisplay())();assert.equal(disconnected.moves.length,0);
});
test('game display following is enabled by default and can be disabled persistently',()=>{
 const f=setup();assert.equal(f.state().followGame,true);f.api.setFollowGame(false);assert.equal(f.state().followGame,false);assert.equal(f.storage.get('gamedeck-follow-display'),'off');assert.equal(setup({'gamedeck-follow-display':'off'}).state().followGame,false);
});
