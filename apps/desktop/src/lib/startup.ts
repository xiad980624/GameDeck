import { invoke, isTauri } from '@tauri-apps/api/core';
const marked=new Set<string>();
export function markStartup(stage:'shell'|'library'|'hero') {
  if(!isTauri()||marked.has(stage))return;
  marked.add(stage);
  void invoke('startup_mark',{stage,frontendMs:performance.now()}).catch(()=>undefined);
}
