import type {InputAction} from './types';
export type ControllerPad=Pick<Gamepad,'id'|'connected'|'axes'|'buttons'>;
export interface NativeGamepadSnapshot{available:boolean;foreground:boolean;wake_held?:boolean;pads:ControllerPad[]}
const buttons:Record<number,InputAction>={0:'CONFIRM',1:'BACK',2:'FAVORITE',3:'MENU',4:'PAGE_PREV',5:'PAGE_NEXT',9:'MENU',12:'UP',13:'DOWN',14:'LEFT',15:'RIGHT'};
export function controllerAction(pads:readonly(ControllerPad|null)[]):InputAction|null{
 for(const pad of pads){if(!pad?.connected)continue;
  for(const [index,action] of Object.entries(buttons)){if(pad.buttons[Number(index)]?.pressed)return action;}
  const x=pad.axes[0]??0,y=pad.axes[1]??0;
  if(Math.max(Math.abs(x),Math.abs(y))>.55)return Math.abs(x)>Math.abs(y)?(x<0?'LEFT':'RIGHT'):(y<0?'UP':'DOWN');
 }return null;
}
export function controllerPads(browser:readonly(ControllerPad|null)[],native:NativeGamepadSnapshot|null):readonly(ControllerPad|null)[]{
 if(!native?.available||!native.pads.some(pad=>pad.connected))return browser;
 // Use one source for Xbox-compatible devices; keep browser support for Sony/DirectInput.
 return [...browser.filter(pad=>pad&&!/xbox|xinput|045e/i.test(pad.id)),...native.pads];
}
export class ControllerRepeat{
 private active:InputAction|null=null;private next=0;private blocked=false;
 suspend(){this.active=null;this.next=0;this.blocked=true;}
 update(action:InputAction|null,time:number):InputAction|null{
  if(!action){this.active=null;this.next=0;this.blocked=false;return null;}
  if(this.blocked)return null;
  if(action!==this.active){this.active=action;this.next=time+250;return action;}
  if(time>=this.next&&['UP','DOWN','LEFT','RIGHT'].includes(action)){this.next=time+90;return action;}
  return null;
 }
}
export function realPointerMove(event:Pick<PointerEvent,'movementX'|'movementY'>){return Math.abs(event.movementX||0)+Math.abs(event.movementY||0)>0;}
