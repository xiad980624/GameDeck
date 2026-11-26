import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));
const ts=require('typescript');
function setup(){
  const pending=new Map();const exports={};let calls=0;
  class Image{src='';decode(){return new Promise(resolve=>pending.set(this.src,resolve));}}
  const modules={
    '@tauri-apps/api/core':{convertFileSrc:path=>'asset:'+path},
    './backend':{desktopMode:true,backend:{heroDisplayPath:async()=>{calls++;return 'C:/display.jpg';}}},
    './store':{useDeck:{}},'./demo':{getHero:g=>g.hero||'fallback'},
  };
  vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/artwork.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports,require:name=>modules[name],Image});
  return {api:exports,pending,calls:()=>calls};
}
test('a scene cannot commit before both background and logo decode',async()=>{
  const {api,pending}=setup();api.primeHeroPaths({'C:/hero.png':'C:/display.jpg'});
  let complete=false;const result=api.prepareScene({id:'death',hero:'C:/hero.png',logo:'C:/logo.png'}).then(value=>{complete=true;return value;});
  pending.get('asset:C:/display.jpg')();await Promise.resolve();await Promise.resolve();assert.equal(complete,false);
  pending.get('asset:C:/logo.png')();const scene=await result;assert.equal(scene.game.id,'death');assert.equal(scene.background,'asset:C:/display.jpg');assert.equal(scene.logo,'asset:C:/logo.png');
});
test('concurrent preloads share work and reuse the display path',async()=>{
  const {api,pending,calls}=setup();const game={id:'cyber',hero:'C:/cyber.png'};
  const a=api.prepareHero(game);const b=api.prepareHero(game);assert.equal(a,b);await new Promise(resolve=>setImmediate(resolve));
  pending.get('asset:C:/display.jpg')();await a;await api.prepareHero(game);assert.equal(calls(),1);assert.equal(api.peekHero(game).src,'asset:C:/display.jpg');
});
test('card source changes render the new source on the same render',()=>{
  const exports={};const modules={
    react:{useState:()=>[null,()=>{}]},'./MotionImage':{MotionImage:'image'},
    'react/jsx-runtime':{jsx:(type,props)=>({type,props})},
    '@tauri-apps/api/core':{convertFileSrc:p=>'asset:'+p},'../lib/backend':{desktopMode:true},
    '../lib/demo':{getLandscape:g=>g.landscape,getHero:g=>g.hero,getPoster:g=>g.poster,getFallbackPoster:()=>'/fallback'},
  };
  vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/components/Artwork.tsx',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX}}).outputText,{exports,require:name=>modules[name]});
  const render=g=>exports.Artwork({game:g,kind:'landscape'}).props.src;
  assert.equal(render({id:'death',landscape:'C:/death.webp'}),'asset:C:/death.webp');
  assert.equal(render({id:'cyber',landscape:'C:/cyber.webp'}),'asset:C:/cyber.webp');
});
