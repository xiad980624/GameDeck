import { useEffect, useId, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';
import type { InputAction } from '../lib/types';

export function DeckDialog({title,onClose,busy=false,children}:{title:string;onClose:()=>void;busy?:boolean;children:ReactNode}) {
  const ref=useRef<HTMLDivElement>(null);
  const id=useId();
  const close=useRef(onClose); close.current=onClose;
  const working=useRef(busy); working.current=busy;
  useEffect(()=>{
    const before=document.activeElement as HTMLElement|null;
    ref.current?.querySelector<HTMLElement>('button:not(:disabled)')?.focus();
    const nodes=()=>Array.from(ref.current?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),summary')??[]).filter(node=>node.getClientRects().length>0);
    const move=(step:number)=>{const list=nodes();const index=list.indexOf(document.activeElement as HTMLElement);list[(index+step+list.length)%list.length]?.focus();};
    const key=(event:KeyboardEvent)=>{
      if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();if(!working.current)close.current();}
      if(event.key==='Tab'){event.preventDefault();event.stopPropagation();move(event.shiftKey?-1:1);}
      if(['ArrowUp','ArrowDown','ArrowLeft','ArrowRight'].includes(event.key)&&!(event.target as HTMLElement)?.matches('input,textarea,select,[role=combobox]')){event.preventDefault();event.stopPropagation();move(event.key==='ArrowDown'||event.key==='ArrowRight'?1:-1);}
    };
    const pad=(event:Event)=>{
      event.preventDefault(); const action=(event as CustomEvent<InputAction>).detail;
      if(action==='BACK'){if(!working.current)close.current();}
      else if(action==='UP'||action==='LEFT')move(-1);
      else if(action==='DOWN'||action==='RIGHT')move(1);
      else if(action==='CONFIRM')(document.activeElement as HTMLElement|null)?.click();
    };
    window.addEventListener('keydown',key,true);window.addEventListener('deck-control',pad);
    return()=>{window.removeEventListener('keydown',key,true);window.removeEventListener('deck-control',pad);if(before?.isConnected)before.focus();};
  },[]);
  return createPortal(<div className="deck-dialog-backdrop"><div className="deck-dialog" ref={ref} role="dialog" aria-modal="true" aria-labelledby={id} data-input-capture>
    <div className="deck-dialog-heading"><h2 id={id}>{title}</h2><button className="icon-button" disabled={busy} aria-label="关闭" onClick={onClose}><X size={20}/></button></div>{children}
  </div></div>,document.body);
}
