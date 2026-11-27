import { invoke, isTauri } from '@tauri-apps/api/core';

type UiSound = 'select' | 'launch';
const preferenceKey = 'gamedeck-ui-sounds';
let enabled: boolean | undefined;
let nextSelectionAt = 0;
let playbackVersion = 0;
const players: Partial<Record<UiSound, HTMLAudioElement>> = {};

export function uiSoundsEnabled(): boolean {
  if (enabled === undefined) {
    try { enabled = localStorage.getItem(preferenceKey) !== 'off'; }
    catch { enabled = true; }
  }
  return enabled;
}

function stopBrowserAudio() {
  for (const player of Object.values(players)) {
    player.pause();
    player.currentTime = 0;
  }
}

export function setUiSoundsEnabled(value: boolean) {
  enabled = value;
  try { localStorage.setItem(preferenceKey, value ? 'on' : 'off'); } catch { /* Session preference still works. */ }
  if (!value) {
    playbackVersion++;
    nextSelectionAt = 0;
    try {
      stopBrowserAudio();
      if (isTauri()) void invoke('play_ui_sound', { sound: null }).catch(() => undefined);
    } catch { /* Audio must never interrupt navigation. */ }
  }
}

function playBrowserAudio(sound: UiSound) {
  try {
    stopBrowserAudio();
    const player = players[sound] ??= new Audio(`/audio/${sound}.wav`);
    // A blocked browser play is discarded, never queued for a later gesture.
    void player.play().catch(() => undefined);
  } catch { /* Missing audio support is harmless. */ }
}

export function playUiSound(sound: UiSound) {
  try {
    const native = isTauri();
    // Native input already checks foreground state; WebView visibility can be stale.
    if (!uiSoundsEnabled() || (!native && document.hidden)) return;
    const now = performance.now();
    if (sound === 'select' && now < nextSelectionAt) return;
    nextSelectionAt = now + (sound === 'launch' ? 350 : 75);
    const version = ++playbackVersion;
    if (native) {
      void invoke<boolean>('play_ui_sound', { sound }).then(handled => {
        if (!handled && version === playbackVersion && uiSoundsEnabled()) playBrowserAudio(sound);
      }).catch(() => undefined);
    } else playBrowserAudio(sound);
  } catch { /* Sound failure cannot block a game launch. */ }
}
