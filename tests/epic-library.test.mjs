import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));const ts=require('typescript');
function backendFixture(){const calls=[];const exports={};const modules={
 '@tauri-apps/api/core':{isTauri:()=>true,invoke:async(name,args)=>{calls.push({name,args});return name==='launch_game'?{state:'awaiting_game'}:undefined;}},
 './uiSounds':{playUiSound:sound=>calls.push({name:'sound',sound})},
 '@tauri-apps/api/event':{listen:async()=>()=>{}},'@tauri-apps/plugin-dialog':{open:async()=>null},
};vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/backend.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports,require:name=>modules[name],Date,localStorage:{getItem:()=>null}});return {backend:exports.backend,calls};}
test('an owned uninstalled Epic game opens installation without a launch installation id',async()=>{
 const {backend,calls}=backendFixture();const session=await backend.launch({id:'owned-epic',platform:'epic',installed:false,launch_installation_id:null},'open-installation');
 assert.equal(calls[0].name,'install_epic_game');assert.equal(calls[0].args.gameId,'owned-epic');assert.equal(session.session_id,'open-installation');assert.equal(session.state,'launcher_started');assert.equal(calls.length,1);
});
test('an installed Epic game uses the monitored game launch command',async()=>{
 const {backend,calls}=backendFixture();const session=await backend.launch({id:'installed-epic',platform:'epic',installed:true,launch_installation_id:'installation'},'launch-request');
 assert.equal(calls[0].name,'sound');assert.equal(calls[0].sound,'launch');assert.equal(calls[1].name,'launch_game');assert.equal(calls[1].args.installationId,'installation');assert.equal(calls[1].args.sessionId,'launch-request');assert.equal(calls[1].args.viaController,false);assert.equal(session.state,'awaiting_game');
 await backend.launch({id:'installed-epic',platform:'epic',installed:true,launch_installation_id:'installation'},'controller-request',true);
 assert.equal(calls[2].sound,'launch');assert.equal(calls[3].args.viaController,true);
});
test('uninstalled games use their platforms installation labels',()=>{
 const exports={};vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/platforms.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText,{exports});
 assert.equal(exports.primaryAction({platform:'epic',installed:false}),'在 Epic 安装');assert.equal(exports.primaryAction({platform:'steam',installed:false}),'在 Steam 安装');assert.equal(exports.primaryAction({platform:'epic',installed:true}),'开始游戏');
});

test('invalid launch entries do not emit a confirmation cue', async () => {
 const {backend,calls}=backendFixture();
 await assert.rejects(backend.launch({id:'invalid',platform:'manual',installed:true,launch_installation_id:null},'invalid'));
 assert.equal(calls.length,0);
});
test('Steam installation is silent like Epic installation', async () => {
 const {backend,calls}=backendFixture();
 await backend.launch({id:'owned-steam',platform:'steam',installed:false,launch_installation_id:null},'install');
 assert.equal(calls.length,1);assert.equal(calls[0].name,'install_steam_game');
});
