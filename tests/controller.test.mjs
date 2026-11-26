import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));const ts=require('typescript');
const compile=file=>ts.transpileModule(readFileSync(new URL('../apps/desktop/src/lib/'+file,import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS}}).outputText;
function module(file,modules={},globals={}){const exports={};vm.runInNewContext(compile(file),{exports,require:name=>modules[name],...globals});return exports;}
const controller=module('controller.ts');const prompts=module('inputPrompts.ts');
const pad=(id='Xbox (XInput)',button=-1,axes=[0,0])=>({id,connected:true,axes,buttons:Array.from({length:16},(_,index)=>({pressed:index===button,touched:index===button,value:index===button?1:0}))});
test('native input is sufficient without a browser-exposed controller or mouse event',()=>{
 const native={available:true,foreground:true,pads:[pad('Xbox (XInput)',0)]};assert.equal(controller.controllerAction(controller.controllerPads([],native)),'CONFIRM');
});

test('a loaded XInput DLL with no native device must not suppress a browser Xbox controller',()=>{
 const pads=controller.controllerPads([pad('Xbox Wireless Controller',0)],{available:true,foreground:true,pads:[]});
 assert.equal(controller.controllerAction(pads),'CONFIRM');
});
test('Xbox uses one state source and Sony browser controllers remain available',()=>{
 const pads=controller.controllerPads([pad('Xbox',0),pad('DualSense')],{available:true,foreground:true,pads:[pad('XInput')]});assert.equal(pads.length,2);assert.equal(controller.controllerAction(pads),null);assert.equal(pads[0].id,'DualSense');
});

async function inputHarness(){
 let clock=0,frame,watchdog,cleanup,receiver,focus,device;
 let snapshot={available:true,foreground:true,pads:[pad()]};let browser=[];let pending;
 const actions=[];let requests=0;let stopped=false;let toggles=0;const keys=[];
 const document={hidden:false,hasFocus:()=>false,activeElement:null,documentElement:{dataset:{}}};
 const input=module('inputDevice.ts',{'./inputPrompts':prompts,zustand:{create:init=>{device=init();return {getState:()=>device,setState:next=>{device={...device,...next};}};}}},{document});
 const globals={document,navigator:{getGamepads:()=>browser},performance:{now:()=>clock},window:{addEventListener:(name,fn)=>{if(name==='keydown')keys.push(fn);},removeEventListener:()=>{}},requestAnimationFrame:fn=>(frame=fn,1),cancelAnimationFrame:()=>{},setInterval:fn=>(watchdog=fn,1),clearInterval:()=>{stopped=true;}};
 const modules={react:{useRef:value=>({current:value}),useEffect:fn=>{cleanup=fn();}},'./inputDevice':input,'./inputPrompts':prompts,'./controller':controller,'@tauri-apps/api/core':{isTauri:()=>true,invoke:()=>{requests++;return pending||Promise.resolve(snapshot);}},'@tauri-apps/api/event':{listen:async(_,fn)=>(receiver=fn,()=>{})},'@tauri-apps/api/window':{getCurrentWindow:()=>({isFocused:async()=>true,onFocusChanged:async fn=>(focus=fn,()=>{})})},'@tauri-apps/api/webview':{getCurrentWebview:()=>({setFocus:async()=>{}})}};
 modules['./window']={toggleFullscreen:async()=>{toggles++;}};
 module('input.ts',modules,globals).useDeckInput(action=>actions.push(action));await new Promise(resolve=>setImmediate(resolve));
 return {actions,document,key(event){keys.forEach(fn=>fn(event));},get toggles(){return toggles;},emit(time,foreground,button=-1,pads=[pad('XInput',button)],wake_held=false){clock=time;snapshot={available:true,foreground,pads,wake_held};receiver({payload:snapshot});},frame(time){clock=time;frame(time);},tick(time){clock=time;watchdog();},status(value){snapshot=value;},browser(value){browser=value;},deferStatus(){pending=new Promise(resolve=>{this.resolveStatus=resolve;});},focus(value){focus({payload:value});},get requests(){return requests;},get device(){return device;},get stopped(){return stopped;},cleanup(){cleanup();}};
}

test('F11 toggles native fullscreen even inside input capture, without repeating while held',async()=>{
 const h=await inputHarness();let prevented=0;h.document.activeElement={closest:()=>true};
 h.key({key:'F11',repeat:false,preventDefault(){prevented++;}});h.key({key:'F11',repeat:true,preventDefault(){prevented++;}});
 assert.equal(h.toggles,1);assert.equal(prevented,2);await new Promise(resolve=>setImmediate(resolve));
 h.key({key:'F11',repeat:false,preventDefault(){}});assert.equal(h.toggles,2);assert.deepEqual(h.actions,[]);h.cleanup();
});

test('native press/release edges work without a render frame, including stale WebView hidden state',async()=>{
 const h=await inputHarness();h.document.hidden=true;
 h.emit(20,true,0);h.emit(30,true);h.emit(40,true,15);
 assert.deepEqual(h.actions,['CONFIRM','RIGHT']);h.cleanup();assert.equal(h.stopped,true);
});

test('background gameplay is ignored and a held exit button is released before lobby input resumes',async()=>{
 const h=await inputHarness();h.emit(20,false,0);h.emit(30,false,15);h.emit(40,true,0);
 assert.deepEqual(h.actions,[]);h.emit(50,true);h.focus(true);h.emit(60,true,0);h.focus(true);h.emit(70,true,0);
 assert.deepEqual(h.actions,['CONFIRM']);h.cleanup();
});

test('watchdog recovers dropped native events with RAF paused and stops after unmount',async()=>{
 const h=await inputHarness();h.status({available:true,foreground:true,pads:[pad('XInput',15)]});
 h.tick(600);await new Promise(resolve=>setImmediate(resolve));assert.deepEqual(h.actions,['RIGHT']);
 h.tick(850);assert.deepEqual(h.actions,['RIGHT','RIGHT']);h.cleanup();const count=h.requests;h.tick(1200);assert.equal(h.requests,count);
});

test('an idle background controller does not swallow the first deliberate press on return',async()=>{
 const h=await inputHarness();h.emit(20,false);h.emit(40,true,0);
 assert.deepEqual(h.actions,['CONFIRM']);h.cleanup();
});

test('a cross-screen wake press cannot also confirm an action after focus returns',async()=>{
 const h=await inputHarness();
 h.emit(20,false);
 h.emit(45,true,0,[pad('XInput',0)],true);
 h.frame(300);
 assert.deepEqual(h.actions,[]);
 h.emit(320,true);
 h.emit(350,true,0);
 assert.deepEqual(h.actions,['CONFIRM']);h.cleanup();
});

test('a held controller direction cannot take back control after a keyboard action',async()=>{
 const h=await inputHarness();h.emit(20,true,15);
 h.key({key:'ArrowLeft',repeat:false,preventDefault(){}});
 h.frame(350);assert.equal(h.device.mode,'keyboard');assert.deepEqual(h.actions,['RIGHT','LEFT']);
 h.emit(400,true);h.emit(420,true,15);
 assert.equal(h.device.mode,'gamepad');assert.deepEqual(h.actions,['RIGHT','LEFT','RIGHT']);h.cleanup();
});

test('an older status response cannot overwrite a newer press event or unblock a held button',async()=>{
 const h=await inputHarness();h.deferStatus();h.tick(600);h.emit(610,true,0);
 h.resolveStatus({available:true,foreground:true,pads:[pad()]});await new Promise(resolve=>setImmediate(resolve));
 h.emit(620,true,0);assert.deepEqual(h.actions,['CONFIRM']);h.cleanup();
});

test('a disconnect and reconnect resumes controls without remounting the input hook',async()=>{
 const h=await inputHarness();h.emit(20,true,0);h.emit(40,true,-1,[]);assert.equal(h.device.mode,'keyboard');
 h.emit(60,true,0);assert.deepEqual(h.actions,['CONFIRM','CONFIRM']);h.cleanup();
});
test('only directions repeat; a held confirm cannot repeatedly activate buttons',()=>{
 const repeat=new controller.ControllerRepeat();assert.equal(repeat.update('CONFIRM',0),'CONFIRM');assert.equal(repeat.update('CONFIRM',600),null);repeat.update(null,610);assert.equal(repeat.update('RIGHT',620),'RIGHT');assert.equal(repeat.update('RIGHT',869),null);assert.equal(repeat.update('RIGHT',870),'RIGHT');assert.equal(repeat.update('RIGHT',959),null);assert.equal(repeat.update('RIGHT',960),'RIGHT');
});
test('returning from a game requires releasing a held button before it can activate the lobby',()=>{
 const repeat=new controller.ControllerRepeat();repeat.suspend();assert.equal(repeat.update('CONFIRM',100),null);repeat.update(null,120);assert.equal(repeat.update('CONFIRM',140),'CONFIRM');
});
test('zero-distance hover and stick noise never count as deliberate navigation',()=>{
 assert.equal(controller.realPointerMove({movementX:0,movementY:0}),false);assert.equal(controller.realPointerMove({movementX:1,movementY:0}),true);assert.equal(controller.controllerAction([pad('Xbox',-1,[.2,-.3])]),null);
});
test('the real input hook accepts foreground native presses with no WebView focus and restores the mouse deliberately',async()=>{
 let clock=0,frame,cleanup,receiver;const events=new Map(),actions=[];const document={hidden:false,hasFocus:()=>false,activeElement:null,documentElement:{dataset:{}}};let device;
 const input=module('inputDevice.ts',{'./inputPrompts':prompts,zustand:{create:init=>{device=init();return {getState:()=>device,setState:next=>{device={...device,...next};}};}}},{document});
 const idle={available:true,foreground:true,pads:[pad()]};
 const globals={document,navigator:{getGamepads:()=>[]},performance:{now:()=>clock},window:{addEventListener:(name,fn)=>{const list=events.get(name)||[];list.push(fn);events.set(name,list);},removeEventListener:()=>{}},requestAnimationFrame:fn=>(frame=fn,1),cancelAnimationFrame:()=>{},setInterval:()=>1,clearInterval:()=>{}};
 const modules={react:{useRef:value=>({current:value}),useEffect:fn=>{cleanup=fn();}},'./inputDevice':input,'./inputPrompts':prompts,'./controller':controller,'@tauri-apps/api/core':{isTauri:()=>true,invoke:async()=>idle},'@tauri-apps/api/event':{listen:async(_,fn)=>(receiver=fn,()=>{})},'@tauri-apps/api/window':{getCurrentWindow:()=>({isFocused:async()=>true,onFocusChanged:async()=>()=>{}})},'@tauri-apps/api/webview':{getCurrentWebview:()=>({setFocus:async()=>{}})}};
 module('input.ts',modules,globals).useDeckInput(action=>actions.push(action));await new Promise(resolve=>setImmediate(resolve));
 const step=time=>{clock=time;frame(time);};const state=(time,foreground,button=-1)=>{clock=time;receiver({payload:{available:true,foreground,pads:foreground?[pad('XInput',button)]:[]}});step(time);};
 step(0);state(20,true,0);assert.deepEqual(actions,['CONFIRM']);assert.equal(document.documentElement.dataset.controllerCursor,'hidden');
 state(100,false,0);state(120,true,0);assert.equal(actions.length,1);state(140,true);state(160,true,0);assert.equal(actions.length,2);
 state(180,true);clock=400;events.get('pointermove')[0]({type:'pointermove',pointerType:'mouse',movementX:0,movementY:0});assert.equal(document.documentElement.dataset.controllerCursor,'hidden');
 events.get('pointermove')[0]({type:'pointermove',pointerType:'mouse',movementX:4,movementY:0});assert.equal(document.documentElement.dataset.controllerCursor,undefined);assert.equal(device.mode,'keyboard');cleanup();
});
