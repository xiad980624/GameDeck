import { create } from 'zustand';
import { keyboardDevice, type InputDevice } from './inputPrompts';

export const useInputDevice = create<InputDevice>(() => keyboardDevice);
export function setInputDevice(next: InputDevice) {
  if(next.mode==='gamepad'&&typeof document!=='undefined')document.documentElement.dataset.controllerCursor='hidden';
  const current = useInputDevice.getState();
  if (current.mode !== next.mode || current.family !== next.family) useInputDevice.setState(next);
}

export function revealPointer(){
 if(typeof document!=='undefined')delete document.documentElement.dataset.controllerCursor;
 setInputDevice(keyboardDevice);
}
