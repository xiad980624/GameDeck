import {useEffect,useRef,useState,type ImgHTMLAttributes} from 'react';
const snapshots=new Map<string,Promise<string>>();
const captureLanes:Promise<unknown>[]=[Promise.resolve(),Promise.resolve()];
let nextLane=0;
export const isMotion=(src:string)=>/\.motion\.(webp|gif)(?:[?#]|$)/i.test(src);
export function useMotionPlayback(){
 const read=()=>localStorage.getItem('gamedeck-motion')==='on'&&!document.hidden&&!matchMedia('(prefers-reduced-motion: reduce)').matches;
 const [enabled,setEnabled]=useState(read);
 useEffect(()=>{const media=matchMedia('(prefers-reduced-motion: reduce)');const update=()=>setEnabled(read());window.addEventListener('gamedeck-motion',update);document.addEventListener('visibilitychange',update);media.addEventListener('change',update);return()=>{window.removeEventListener('gamedeck-motion',update);document.removeEventListener('visibilitychange',update);media.removeEventListener('change',update);};},[]);
 return enabled;
}
function still(src:string,maxWidth:number):Promise<string>{
 const key=maxWidth+':'+src;const cached=snapshots.get(key);if(cached)return cached;
 const lane=nextLane++%captureLanes.length;
 const task=captureLanes[lane].then(()=>new Promise<string>((resolve,reject)=>{
  const img=new Image();img.crossOrigin='anonymous';
  img.onload=()=>{
   try{
    const scale=Math.min(1,maxWidth/img.naturalWidth);const canvas=document.createElement('canvas');
    canvas.width=Math.max(1,Math.round(img.naturalWidth*scale));canvas.height=Math.max(1,Math.round(img.naturalHeight*scale));
    const ctx=canvas.getContext('2d');if(!ctx)throw Error('canvas unavailable');
    ctx.drawImage(img,0,0,canvas.width,canvas.height);
    // Synchronous encoding of large animated frames stalls input and scrolling.
    canvas.toBlob(blob=>{
     if(!blob){reject(Error('frame encoding failed'));return;}
     const reader=new FileReader();reader.onload=()=>resolve(String(reader.result));reader.onerror=()=>reject(Error('frame reading failed'));reader.readAsDataURL(blob);
    },'image/webp',.9);
   }catch(error){reject(error);}finally{img.onload=null;img.onerror=null;img.src='';}
  };
  img.onerror=()=>{img.onload=null;img.onerror=null;img.src='';reject(Error('frame loading failed'));};
  img.src=src;
 }));
 captureLanes[lane]=task.catch(()=>undefined);
 snapshots.set(key,task);void task.catch(()=>snapshots.delete(key));
 while(snapshots.size>12)snapshots.delete(snapshots.keys().next().value!);return task;
}
export function MotionImage({src='',play=true,maxStillWidth=2560,...props}:ImgHTMLAttributes<HTMLImageElement>&{play?:boolean;maxStillWidth?:number}){
 const enabled=useMotionPlayback();const frozen=isMotion(src)&&(!enabled||!play);
 const element=useRef<HTMLImageElement>(null);const [visible,setVisible]=useState(false);
 const deferred=frozen&&props.loading==='lazy'&&!visible;
 useEffect(()=>{
  if(!deferred)return;
  if(typeof IntersectionObserver==='undefined'){setVisible(true);return;}
  const observer=new IntersectionObserver(entries=>{if(entries.some(entry=>entry.isIntersecting)){setVisible(true);observer.disconnect();}},{rootMargin:'160px'});
  if(element.current)observer.observe(element.current);return()=>observer.disconnect();
 },[deferred]);
 const [frame,setFrame]=useState({source:'',url:''});
 useEffect(()=>{let active=true;if(frozen&&!deferred)void still(src,maxStillWidth).then(url=>{if(active)setFrame({source:src,url});}).catch(()=>{if(active)setFrame({source:src,url:'/art/fallback-hero.svg'});});return()=>{active=false;};},[src,frozen,deferred,maxStillWidth]);
 return <img {...props} ref={element} src={frozen?(frame.source===src?frame.url:'/art/fallback-hero.svg'):src}/>;
}
