export type InputDevice = { mode: 'keyboard' | 'gamepad'; family: 'xbox' | 'playstation' };
export const keyboardDevice: InputDevice = { mode: 'keyboard', family: 'xbox' };
type Pad = Pick<Gamepad, 'id' | 'connected' | 'axes' | 'buttons'>;

export function detectInputDevice(current: InputDevice, pads: readonly (Pad | null)[]): InputDevice {
  const connected = pads.filter((pad): pad is Pad => !!pad?.connected);
  const active = connected.find(pad => pad.axes.some(axis => Math.abs(axis) > 0.55) || pad.buttons.some(button => button.pressed || button.value > 0.5));
  if (active) return { mode: 'gamepad', family: /dualsense|dualshock|playstation|054c|sony|^wireless controller/i.test(active.id) ? 'playstation' : 'xbox' };
  return current.mode === 'gamepad' && !connected.length ? keyboardDevice : current;
}

export function inputLabels(device: InputDevice) {
  if (device.mode === 'keyboard') return { move: '方向键', confirm: 'Enter', back: 'Esc', favorite: 'X', menu: 'Y', prev: 'Q', next: 'E' };
  return device.family === 'playstation'
    ? { move: '十字键 / 左摇杆', confirm: '×', back: '○', favorite: '□', menu: '△', prev: 'L1', next: 'R1' }
    : { move: '十字键 / 左摇杆', confirm: 'A', back: 'B', favorite: 'X', menu: 'Y', prev: 'LB', next: 'RB' };
}
