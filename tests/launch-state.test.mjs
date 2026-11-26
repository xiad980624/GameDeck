import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));const ts=require('typescript');
const api={};
vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/launchState.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports:api});
const session=(state,id='real')=>({state,session_id:id,game_id:'epic-game',timestamp:'2026-11-21'});
test('a client handoff remains pending until the real process event arrives',()=>{
 assert.equal(api.launchPending(session('preparing')),true);assert.equal(api.launchPending(session('awaiting_game')),true);
 const running=api.nextSession(session('awaiting_game'),session('running'));assert.equal(running.state,'running');assert.equal(api.launchPending(running),false);
 assert.equal(api.nextSession(running,session('awaiting_game')).state,'running');
});
test('terminal sessions cannot be revived by stale events',()=>{
 for(const state of ['failed','exited'])assert.equal(api.nextSession(session(state),session('running')).state,state);
});
test('only updates carrying the explicitly opened session ID can affect a retry',()=>{
 const retry=session('preparing','retry');
 assert.equal(api.nextSession(undefined,session('running')),undefined);
 assert.equal(api.nextSession(retry,session('failed')),retry);
 const running=api.nextSession(retry,session('running','retry'));
 assert.equal(api.nextSession(running,session('exited')),running);
 assert.equal(api.nextSession(running,session('exited','retry')).state,'exited');
});
test('the primary button shows a spinner while pending and running only for the running state',()=>{
 const exports={};let current;const element=(type,props)=>({type,props});
 const modules={'react/jsx-runtime':{jsx:element,jsxs:element},'lucide-react':{LoaderCircle:'spinner',MonitorPlay:'monitor',Play:'play'},'../lib/store':{useDeck:selector=>selector({sessions:{'epic-game':current}})},'../lib/launchState':api,'../lib/platforms':{primaryAction:()=> '开始游戏'}};
 vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/components/LaunchButton.tsx',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX}}).outputText,{exports,require:name=>modules[name]});
 for(const [state,label,icon,busy] of [['preparing','启动中…','spinner',true],['awaiting_game','启动中…','spinner',true],['running','运行中','monitor',false]]){
  current=session(state);const view=exports.LaunchButton({game:{id:'epic-game'},onLaunch:()=>{}});const button=view.props.children[0];assert.equal(button.props.children[0].type,icon);assert.equal(button.props.children[1].props.children,label);assert.equal(button.props['aria-busy'],busy);assert.equal(button.props.disabled,true);
 }
});
