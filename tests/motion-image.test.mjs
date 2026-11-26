import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require=createRequire(new URL('../apps/desktop/package.json',import.meta.url));
const ts=require('typescript');
const flush=()=>new Promise(resolve=>setImmediate(resolve));
function setup(){
 const exports={},images=[],encodes=[],observers=[];let current;
 const state=initial=>{const index=current.index++;if(!(index in current.slots))current.slots[index]=typeof initial==='function'?initial():initial;const owner=current;return [owner.slots[index],value=>{owner.slots[index]=typeof value==='function'?value(owner.slots[index]):value;}];};
 const react={useState:state,useRef:value=>state({current:value})[0],useEffect:(run,deps)=>{const index=current.index++;const previous=current.slots[index];if(!previous||deps.some((value,i)=>value!==previous.deps[i])){previous?.cleanup?.();const effect={deps,run};current.slots[index]=effect;current.effects.push(effect);}}};
 class Image{naturalWidth=3840;naturalHeight=2160;set src(value){this.source=value;if(value)images.push(this);}get src(){return this.source;}}
 class Reader{readAsDataURL(){this.result='data:image/webp;base64,frame';this.onload();}}
 class Observer{constructor(callback){this.callback=callback;observers.push(this);}observe(){}disconnect(){}visible(){this.callback([{isIntersecting:true}]);}}
 const document={hidden:false,addEventListener(){},removeEventListener(){},createElement(){const canvas={width:0,height:0,getContext:()=>({drawImage(){}}),toBlob(callback){encodes.push({width:canvas.width,height:canvas.height,finish:()=>callback({})});},toDataURL(){throw Error('synchronous encoding must not run');}};return canvas;}};
 const modules={react,'react/jsx-runtime':{jsx:(type,props)=>({type,props})}};
 vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/components/MotionImage.tsx',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX}}).outputText,{exports,require:name=>modules[name],Image,FileReader:Reader,IntersectionObserver:Observer,document,window:{addEventListener(){},removeEventListener(){}},localStorage:{getItem:()=>null},matchMedia:()=>({matches:false,addEventListener(){},removeEventListener(){}})});
 const instance=props=>{const owner={slots:[],index:0,effects:[]};return {render(){current=owner;owner.index=0;owner.effects=[];const result=exports.MotionImage(props);result.props.ref.current={};for(const effect of owner.effects)effect.cleanup=effect.run();return result;}};};
 return {images,encodes,observers,instance};
}

test('offscreen animated thumbnails do not start decoding before entering the viewport',async()=>{
 const h=setup();const card=h.instance({src:'card.motion.webp',loading:'lazy',play:false,maxStillWidth:640});
 card.render();await flush();assert.equal(h.images.length,0);h.observers[0].visible();card.render();await flush();assert.equal(h.images.length,1);
 h.images[0].onload();assert.equal(h.encodes[0].width,640);assert.equal(h.encodes[0].height,360);h.encodes[0].finish();await flush();assert.equal(card.render().props.src,'data:image/webp;base64,frame');
});

test('animated frame work is limited to two images and encoding yields to the browser',async()=>{
 const h=setup();for(let i=0;i<5;i++)h.instance({src:i+'.motion.webp',play:false,maxStillWidth:640}).render();
 await flush();assert.equal(h.images.length,2);h.images[0].onload();assert.equal(h.encodes.length,1);assert.equal(h.images.length,2);
 h.encodes[0].finish();await flush();assert.equal(h.images.length,3);
});

test('thumbnail and hero captures use independent sizes and repeated thumbnail requests share work',async()=>{
 const h=setup();const src='same.motion.webp';h.instance({src,play:false,maxStillWidth:640}).render();h.instance({src,play:false,maxStillWidth:640}).render();h.instance({src,play:false,maxStillWidth:2560}).render();
 await flush();assert.equal(h.images.length,2);h.images.forEach(image=>image.onload());assert.deepEqual(h.encodes.map(item=>item.width).sort((a,b)=>a-b),[640,2560]);h.encodes.forEach(item=>item.finish());await flush();
});
