import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';

export const useWindowState = create(() => ({ fullscreen: true, maximized: false }));
let syncRevision = 0;

export async function syncWindowState() {
  const revision = ++syncRevision;
  const win = getCurrentWindow();
  const [fullscreen, maximized] = await Promise.all([win.isFullscreen(), win.isMaximized()]);
  if (revision === syncRevision) useWindowState.setState({ fullscreen, maximized });
}

export async function toggleFullscreen() {
  await invoke('toggle_lobby_fullscreen');
  await syncWindowState();
}

export async function toggleMaximize() {
  const win = getCurrentWindow();
  if (await win.isFullscreen()) {
    await win.setFullscreen(false);
    await win.unmaximize();
  } else {
    await win.toggleMaximize();
  }
  await syncWindowState();
}
