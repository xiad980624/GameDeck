import { useEffect, useRef } from 'react';
import type { InputAction } from './types';
import { revealPointer,setInputDevice, useInputDevice } from './inputDevice';
import { detectInputDevice, keyboardDevice } from './inputPrompts';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {getCurrentWebview} from '@tauri-apps/api/webview';
import {ControllerRepeat,controllerAction,controllerPads,realPointerMove,type NativeGamepadSnapshot} from './controller';
import {toggleFullscreen} from './window';

/**
 * 统一键盘与 Gamepad API 输入；不依赖浏览器 DOM 原生焦点来导航虚拟列表。
 * 长按节流：首按即时、250ms 后以 90ms 连发；多个手柄以第一个有效输入为准。
 */
export function useDeckInput(onAction: (a: InputAction) => void) {
  const handler = useRef(onAction);
  handler.current = onAction;
  useEffect(() => {
    let fullscreenBusy=false;
    let controllerActionHeld: InputAction | null = null;
    let controllerWasActive = false;
    const repeater=new ControllerRepeat();
    const takeKeyboard = () => {
      if (controllerActionHeld) repeater.suspend();
      setInputDevice(keyboardDevice);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.altKey || event.metaKey) return;
      const element = document.activeElement as HTMLElement | null;
      if (element?.closest('[data-input-capture]')) return;
      const caption = element?.closest('[data-window-controls]');
      if (caption && ['Enter', ' ', 'Tab'].includes(event.key)) return;
      if (element?.matches('input, textarea, select, [contenteditable="true"]')) {
        if (event.key === 'Escape') { element.blur(); handler.current('BACK'); event.preventDefault(); }
        return;
      }
      const actions: Record<string, InputAction> = {
        ArrowUp: 'UP', ArrowDown: 'DOWN', ArrowLeft: 'LEFT', ArrowRight: 'RIGHT',
        Enter: 'CONFIRM', ' ': 'CONFIRM', Escape: 'BACK', Backspace: 'BACK',
        x: 'FAVORITE', X: 'FAVORITE', y: 'MENU', Y: 'MENU',
        q: 'PAGE_PREV', Q: 'PAGE_PREV', e: 'PAGE_NEXT', E: 'PAGE_NEXT',
        '/': 'SEARCH',
      };
      const action = actions[event.key];
      if (action) {
        if (caption) element?.blur();
        event.preventDefault();
        handler.current(action);
      }
    };
    const intent = (event: KeyboardEvent) => {
      if (!event.repeat && !['Shift','Control','Alt','Meta'].includes(event.key)) takeKeyboard();
      if (event.key !== 'F11' || event.ctrlKey || event.altKey || event.metaKey || !isTauri()) return;
      event.preventDefault();
      event.stopPropagation?.();
      if (!event.repeat && !fullscreenBusy) {
        fullscreenBusy = true;
        void toggleFullscreen().catch(() => undefined).finally(() => { fullscreenBusy = false; });
      }
    };
    window.addEventListener('keydown', intent, true);
    window.addEventListener('keydown', onKey);
    let pointerDistance=0;let native:NativeGamepadSnapshot|null=null;let nativeAt=-Infinity;
    let disposed=false;const cleanups:(()=>void)[]=[];
    const takePointer = () => {
      pointerDistance=0;
      if (controllerActionHeld) repeater.suspend();
      revealPointer();
    };
    const onPointer=(event:PointerEvent)=>{
      if(event.pointerType&&event.pointerType!=='mouse')return;
      if(event.type==='pointerdown'){takePointer();return;}
      if(!realPointerMove(event))return;
      pointerDistance+=Math.abs(event.movementX)+Math.abs(event.movementY);
      if(pointerDistance>=3)takePointer();
    };
    const onWheel=(event:WheelEvent)=>{if(!event.ctrlKey&&(event.deltaX||event.deltaY))takePointer();};
    window.addEventListener('pointerdown',onPointer,true);
    window.addEventListener('pointermove',onPointer,true);
    window.addEventListener('wheel',onWheel,{capture:true,passive:true});
    const sample=(time:number)=>{
      if(disposed)return;
      const fresh=native&&time-nativeAt<1000?native:null;
      const focused=fresh?fresh.foreground:document.hasFocus();
      // Native foreground state remains authoritative even when WebView visibility
      // or document focus has not caught up after a full-screen game exits.
      if(focused&&(fresh||!document.hidden)){
        let browser:readonly(Gamepad|null)[]=[];try{browser=Array.from(navigator.getGamepads?.()??[]);}catch{/* Native input remains available if the WebView has not exposed Gamepad API. */}
        const pads=controllerPads(browser,fresh);
        const current=useInputDevice.getState();
        const active=detectInputDevice(keyboardDevice,pads);
        const action=controllerAction(pads);
        if (active.mode==='gamepad' && (current.mode==='gamepad'||!controllerWasActive||(action&&action!==controllerActionHeld))) {
          setInputDevice(active);
          if (current.mode==='keyboard') pointerDistance=0;
        } else if (current.mode==='gamepad' && !pads.some(pad=>pad?.connected)) {
          setInputDevice(keyboardDevice);
        }
        controllerWasActive=active.mode==='gamepad';
        controllerActionHeld=action;
        if (fresh?.wake_held) {
          repeater.suspend();
          return;
        }
        const fired=repeater.update(action,time);if(fired)handler.current(fired);
      }else{
        repeater.suspend();
        // Background neutral snapshots can arm the next deliberate press. A
        // button held across the return remains blocked until it is released.
        if(fresh?.pads.some(pad=>pad.connected)&&!controllerAction(fresh.pads)){
          controllerActionHeld=null;controllerWasActive=false;repeater.update(null,time);
        }
      }
    };
    let nativeRevision=0;
    const receive=(snapshot:NativeGamepadSnapshot)=>{
      if(disposed)return;
      // Consume press/release edges directly; RAF can pause while a game covers
      // the WebView, and a short press must not disappear between two frames.
      nativeRevision++;native=snapshot;nativeAt=performance.now();sample(nativeAt);
    };
    let requesting=false;let lastRequest=-Infinity;
    const refreshNative=()=>{
      if(disposed||requesting)return;
      requesting=true;lastRequest=performance.now();const revision=nativeRevision;
      void invoke<NativeGamepadSnapshot>('native_gamepad_status').then(snapshot=>{
        // A command response may be older than an event received in flight.
        if(nativeRevision===revision)receive(snapshot);
      }).catch(()=>undefined).finally(()=>{requesting=false;});
    };
    const desktop=isTauri();
    if(desktop){
      void listen<NativeGamepadSnapshot>('native-gamepad-state',event=>receive(event.payload)).then(fn=>{if(disposed)fn();else cleanups.push(fn);}).catch(()=>undefined);
      refreshNative();
      const focusWebview=()=>{void getCurrentWebview().setFocus().catch(()=>undefined);};
      const win=getCurrentWindow();
      void win.isFocused().then(focused=>{if(focused&&!disposed)focusWebview();}).catch(()=>undefined);
      void win.onFocusChanged(event=>{
        // Focusing the embedded WebView can produce another focus notification.
        // Only a real loss of focus should block held buttons until release.
        if(!event.payload){if(native)native={...native,foreground:false};repeater.suspend();}
        else{if(!document.hasFocus())focusWebview();refreshNative();}
      }).then(fn=>{if(disposed)fn();else cleanups.push(fn);}).catch(()=>undefined);
    }
    let raf=0;let lastFrame=-Infinity;
    const poll=(time:number)=>{
      lastFrame=time;sample(time);
      raf=requestAnimationFrame(poll);
    };
    // Recover input without mouse movement even if Chromium has suspended RAF.
    const watchdog=setInterval(()=>{
      const time=performance.now();
      if(desktop&&time-lastRequest>=500)refreshNative();
      if(time-lastFrame>=80)sample(time);
    },50);
    raf = requestAnimationFrame(poll);
    return()=>{disposed=true;clearInterval(watchdog);cleanups.forEach(fn=>fn());window.removeEventListener('keydown',onKey);window.removeEventListener('keydown',intent,true);window.removeEventListener('pointerdown',onPointer,true);window.removeEventListener('pointermove',onPointer,true);window.removeEventListener('wheel',onWheel,true);cancelAnimationFrame(raf);};
  }, []);
}
