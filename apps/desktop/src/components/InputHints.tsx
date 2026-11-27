import { useInputDevice } from '../lib/inputDevice';
import { inputLabels } from '../lib/inputPrompts';
import type { Page } from '../lib/types';

export function InputHints({ context }: { context: Page }) {
  const device = useInputDevice();
  const key = inputLabels(device);
  const hints = [
    [key.move, '选择'],
    [key.confirm, context === 'details' ? '确认' : context === 'settings' || context === 'accelerators' ? '确认' : '详情'],
    ...(context !== 'home' ? [[key.back, '返回']] : []),
    ...(context !== 'settings' && context !== 'accelerators' ? [[key.favorite, '收藏']] : []),
    ...(context === 'home' ? [[key.menu, '设置'], [key.prev + ' / ' + key.next, '游戏库']] : []),
    ...(context === 'settings' ? [[key.prev + ' / ' + key.next, '页签']] : []),
    ...(context === 'accelerators' ? [[key.prev + ' / ' + key.next, '加速器']] : []),
    ...(context === 'details' ? [[key.menu, '更多']] : []),
    ...(context === 'library' ? [[key.menu, '排序'], [key.prev + ' / ' + key.next, '平台']] : []),
  ];
  return <span className={`input-hints input-${device.mode}`} aria-label={device.mode === 'gamepad' ? '手柄操作' : '键盘操作'}>
    {hints.map(([button,action]) => <span className="input-hint" key={action}><kbd>{button}</kbd>{action}</span>)}
  </span>;
}
