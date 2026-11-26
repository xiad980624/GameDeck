import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));const ts=require('typescript');
function backendFixture(){const calls=[];const exports={};const modules={
 '@tauri-apps/api/core':{isTauri:()=>true,invoke:async(name,args)=>{calls.push({name,args});return name==='launch_game'?{state:'awaiting_game'}:undefined;}},
 '@tauri-apps/api/event':{listen:async()=>()=>{}},'@tauri-apps/plugin-dialog':{open:async()=>null},
};vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/backend.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports,require:name=>modules[name],Date,localStorage:{getItem:()=>null}});return {backend:exports.backend,calls};}
test('an owned uninstalled Epic game opens installation without a launch installation id',async()=>{
 const {backend,calls}=backendFixture();const session=await backend.launch({id:'owned-epic',platform:'epic',installed:false,launch_installation_id:null},'open-installation');
 assert.equal(calls[0].name,'install_epic_game');assert.equal(calls[0].args.gameId,'owned-epic');assert.equal(session.session_id,'open-installation');assert.equal(session.state,'launcher_started');assert.equal(calls.length,1);
});
test('an installed Epic game uses the monitored game launch command',async()=>{
 const {backend,calls}=backendFixture();const session=await backend.launch({id:'installed-epic',platform:'epic',installed:true,launch_installation_id:'installation'},'launch-request');
 assert.equal(calls[0].name,'launch_game');assert.equal(calls[0].args.installationId,'installation');assert.equal(calls[0].args.sessionId,'launch-request');assert.equal(calls[0].args.viaController,false);assert.equal(session.state,'awaiting_game');
 await backend.launch({id:'installed-epic',platform:'epic',installed:true,launch_installation_id:'installation'},'controller-request',true);
 assert.equal(calls[1].args.viaController,true);
});
test('uninstalled games use their platforms installation labels',()=>{
 const exports={};vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/platforms.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports});
 assert.equal(exports.primaryAction({platform:'epic',installed:false}),'在 Epic 安装');assert.equal(exports.primaryAction({platform:'steam',installed:false}),'在 Steam 安装');assert.equal(exports.primaryAction({platform:'epic',installed:true}),'开始游戏');
});
