import type { Game } from './types';
export const platformName=(id:string)=>({all:'全部平台',steam:'Steam',epic:'Epic Games',battlenet:'战网',manual:'本地游戏',kuro:'库洛'}[id]||id);
export const updateLabel=(game:Game)=>({unknown:'需客户端检查',not_installed:'未安装',not_required:'本地无待更新',update_available:'需要更新',updating:'更新中',paused:'更新已暂停',repair_required:'需要校验',installing:'安装/更新中'}[game.update_status||'unknown']);
export const updatePending=(game:Game)=>['update_available','updating','paused','repair_required','installing'].includes(game.update_status||'');

export const primaryAction=(game:Game)=>game.installed?'开始游戏':game.platform==='steam'?'在 Steam 安装':game.platform==='epic'?'在 Epic 安装':'打开客户端';

export const libraryPlatforms=(games:Game[])=>['all','steam','epic','battlenet','manual',...new Set(games.map(g=>g.platform).filter(p=>!['steam','epic','battlenet','manual'].includes(p)))];
