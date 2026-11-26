import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));
const ts=require('typescript');
const cache=new Map();
const memory=new Map();
const localStorage={getItem:key=>memory.get(key)??null,setItem:(key,value)=>memory.set(key,value)};
function load(name){
  if(cache.has(name))return cache.get(name);
  const exports={};cache.set(name,exports);
  const customRequire=id=>id==='@tauri-apps/api/core'?{isTauri:()=>true}:id.startsWith('./')?load(id.slice(2)):require(id);
  vm.runInNewContext(ts.transpileModule(readFileSync(new URL(`../apps/desktop/src/lib/${name}.ts`,import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports,require:customRequire,localStorage});
  return exports;
}
const {spaceGames,libraryGames,hiddenShortcut}=load('libraryScope');
const {homeGames}=load('homeOrder');
const {useDeck}=load('store');
const game=(id,hidden=false)=>({id,name:id,original_name:id,hidden,platform:'manual',genre:'游戏',favorite:true,installed:true,total_minutes:0,added_at:'2026-11-21',last_played_at:null});
const games=[game('public'),game('private',true)];
test('hidden favorite and newly added games never enter home or normal search',()=>{
  assert.equal(homeGames(games,['private']).map(g=>g.id).join(','),'public');
  assert.equal(spaceGames(games).map(g=>g.id).join(','),'public');
  assert.equal(spaceGames(games,true).map(g=>g.id).join(','),'private');
  assert.equal(libraryGames({games,hiddenSpace:false,query:'private',platformFilter:'all',filter:'favorites',sort:'recent'}).length,0);
  assert.equal(libraryGames({games,hiddenSpace:true,query:'private',platformFilter:'all',filter:'favorites',sort:'recent'})[0].id,'private');
});
test('shortcut is limited to library and ignores editing, repeats and IME',()=>{
  const event={code:'KeyH',ctrlKey:true,shiftKey:true,altKey:false,metaKey:false,repeat:false,isComposing:false};
  assert.equal(hiddenShortcut(event,'library',false),true);
  for(const page of ['home','details','settings','accelerators'])assert.equal(hiddenShortcut(event,page,false),false);
  assert.equal(hiddenShortcut(event,'library',true),false);
  for(const property of ['repeat','isComposing','altKey','metaKey'])assert.equal(hiddenShortcut({...event,[property]:true},'library',false),false);
  assert.equal(hiddenShortcut({...event,shiftKey:false},'library',false),false);
});
test('navigation closes hidden session while detail and game accelerator round trip preserve it',()=>{
  useDeck.getState().setGames(games);
  useDeck.getState().toggleHiddenSpace();assert.equal(useDeck.getState().hiddenSpace,false);
  useDeck.getState().setPage('library');useDeck.getState().toggleHiddenSpace();
  assert.equal(useDeck.getState().hiddenSpace,true);assert.equal(useDeck.getState().focusedId,'private');
  useDeck.getState().showDetail('public');assert.equal(useDeck.getState().page,'library');
  useDeck.getState().showDetail('private');assert.equal(useDeck.getState().page,'details');
  useDeck.getState().openAccelerators('uu','private');assert.equal(useDeck.getState().hiddenSpace,true);
  useDeck.getState().setPage('details');useDeck.getState().closeDetail();
  assert.equal(useDeck.getState().page,'library');assert.equal(useDeck.getState().hiddenSpace,true);
  useDeck.getState().setPage('home');assert.equal(useDeck.getState().hiddenSpace,false);assert.equal(useDeck.getState().focusedId,'public');
  useDeck.getState().setPage('library');assert.equal(useDeck.getState().hiddenSpace,false);
  useDeck.getState().toggleHiddenSpace();useDeck.getState().setPage('settings');assert.equal(useDeck.getState().hiddenSpace,false);
});
