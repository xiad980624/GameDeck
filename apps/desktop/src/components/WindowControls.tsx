import { useCallback, useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { Copy, Maximize, Minus, Square, X } from 'lucide-react';
import { useDeck } from '../lib/store';
import { syncWindowState, toggleFullscreen, toggleMaximize, useWindowState } from '../lib/window';
import { useInputDevice } from '../lib/inputDevice';

export function WindowControls() {
  const inputMode = useInputDevice(s => s.mode);
  const { fullscreen, maximized } = useWindowState();
  const [pending, setPending] = useState(false);
  const run = useCallback(async (action: () => Promise<unknown>) => {
    setPending(true);
    try { await action(); }
    catch (error) { useDeck.getState().notify(`窗口操作失败：${String(error)}`); }
    finally { setPending(false); }
  }, []);

  useEffect(() => {
    let disposed = false;
    const cleanups: (() => void)[] = [];
    const refresh = () => { void syncWindowState().catch(() => undefined); };
    const win = getCurrentWindow();
    refresh();
    for (const subscription of [win.onResized(refresh), win.onFocusChanged(refresh), listen('lobby-window-mode-changed', refresh)]) {
      void subscription.then(unlisten => {
        if (disposed) unlisten(); else cleanups.push(unlisten);
      }).catch(() => undefined);
    }
    return () => {
      disposed = true;
      cleanups.forEach(unlisten => unlisten());
    };
  }, []);

  if(fullscreen)return null;
  const restore = maximized;
  return <div className="window-controls" data-window-controls role="group" aria-label="窗口控制">
    <button className="window-control window-fullscreen" disabled={pending} onClick={() => void run(toggleFullscreen)} aria-label="进入全屏" title={inputMode === 'keyboard' ? '进入全屏 (F11)' : '进入全屏'}><Maximize size={16}/></button>
    <button className="window-control" disabled={pending} onClick={() => void run(() => getCurrentWindow().minimize())} aria-label="最小化" title="最小化"><Minus size={16}/></button>
    <button className="window-control" disabled={pending} onClick={() => void run(toggleMaximize)} aria-label={restore ? '还原窗口' : '最大化'} title={restore ? '还原窗口' : '最大化'}>{restore ? <Copy size={14}/> : <Square size={13}/>}</button>
    <button className="window-control window-close" onClick={() => void run(() => getCurrentWindow().close())} aria-label="关闭窗口" title="关闭"><X size={18}/></button>
  </div>;
}
