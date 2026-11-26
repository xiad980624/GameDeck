import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { playUiSound } from './uiSounds';
import type { Game, ScanReport, SessionInfo } from './types';

export const desktopMode = isTauri();
const localKey = 'gamedeck-web-library';

export interface SteamAccounts { accounts: {id:string;name:string;most_recent:boolean}[]; selected:string|null; scope:'cache'|'owned' }

export interface Accelerator {id:'uu'|'xunyou';name:string;path:string|null;website:string}
export interface ManualCandidate { path: string; relative_path: string; size: number; score: number }
export interface ManualFolderScan { name: string; candidates: ManualCandidate[]; warnings: string[] }
export interface EpicAccountStatus {connected:boolean;display_name:string;count:number;synced_at:string|null;syncing:boolean}
export interface EpicLibrarySync {count:number;inserted:number;unresolved:number;skipped:number}
export interface EpicArtworkReport {processed:number;total:number;saved:number;skipped?:number;errors?:string[];games?:Record<string,{game:string;missing:string[];errors:string[];saved:number}>}
export interface EpicArtworkStatus {running:boolean;report:EpicArtworkReport|null;error?:string|null}
export const backend = {
  async repairEpicArtwork(retry=false):Promise<boolean>{return desktopMode?invoke('repair_epic_artwork',{retry}):false;},
  async epicArtworkStatus():Promise<EpicArtworkStatus>{return desktopMode?invoke('epic_artwork_status'):{running:false,report:null};},
  async subscribeEpicArtwork(cb:()=>void):Promise<()=>void>{return desktopMode?listen('epic-artwork-progress',()=>cb()):()=>undefined;},
  async epicAccountStatus():Promise<EpicAccountStatus>{return desktopMode?invoke('epic_account_status'):{connected:false,display_name:'',count:0,synced_at:null,syncing:false};},
  async openEpicLogin():Promise<void>{return invoke('open_epic_login');},
  async connectEpicAccount(code:string):Promise<EpicLibrarySync>{return invoke('connect_epic_account',{code});},
  async syncEpicLibrary():Promise<EpicLibrarySync>{return invoke('sync_epic_library');},
  async disconnectEpicAccount():Promise<void>{return invoke('disconnect_epic_account');},
  async subscribeLibraryChanged(cb:()=>void):Promise<()=>void>{return desktopMode?listen('game-library-changed',()=>cb()):()=>undefined;},
  async repairTitles():Promise<Game[]>{return desktopMode?invoke('repair_titles'):this.listGames();},
  async subscribeScanProgress(cb:(label:string)=>void):Promise<()=>void>{return desktopMode?listen<string>('library-scan-progress',event=>cb(event.payload)):()=>undefined;},
  async refreshGameStatus(gameId:string):Promise<Game|null>{return desktopMode?invoke('refresh_game_status',{gameId}):null;},
  async startupLibrary():Promise<{games:Game[];hero_displays:Record<string,string>;probe:boolean;navigation_probe?:boolean}>{
    return desktopMode?invoke('startup_library'):{games:await this.listGames(),hero_displays:{},probe:false};
  },
  async heroDisplayPath(gameId:string):Promise<string|null>{return desktopMode?invoke('hero_display_path',{gameId}):null;},
  async sgdbKeyStatus():Promise<boolean>{return desktopMode?invoke('sgdb_key_status'):false;},
  async setSgdbKey(key:string):Promise<void>{return invoke('set_sgdb_key',{key});},
  async alphaKeyStatus():Promise<boolean>{return desktopMode?invoke('alpha_key_status'):false;},
  async setAlphaKey(key:string):Promise<void>{return invoke('set_alpha_key',{key});},
  async refreshPromoBackgrounds():Promise<{processed:number;saved:number;entries:{game:string;status:string;selection?:{tier:string;width:number;height:number};reason?:string;audit?:{issues?:string[];candidates?:{decision?:string;reason?:string}[]}}[];errors:string[]}>{return invoke('refresh_promo_backgrounds');},
  async improveBackground(gameId:string):Promise<Game[]>{if(!desktopMode)throw new Error('高清背景更新需要桌面版');return invoke('improve_background',{gameId,apiKey:null});},
  async accelerators():Promise<Accelerator[]> { return desktopMode?invoke('list_accelerators'):[{id:'uu',name:'UU加速器',path:null,website:'https://uu.163.com/'},{id:'xunyou',name:'迅游加速器',path:null,website:'https://www.xunyou.com/'}]; },
  async openAccelerator(provider:string,website=false):Promise<void>{if(!desktopMode)throw new Error('请在 Windows 桌面版打开加速器');return invoke('open_accelerator',{provider,website});},
  async chooseAccelerator(provider:string):Promise<boolean>{const path=await open({multiple:false,directory:false,filters:[{name:'加速器程序',extensions:['exe']}]});if(!path)return false;await invoke('set_accelerator_path',{provider,path});return true;},
  async resetAccelerator(provider:string):Promise<void>{return invoke('set_accelerator_path',{provider,path:null});},
  async listGames(): Promise<Game[]> {
    if (desktopMode) return invoke<Game[]>('list_games');
    try { return JSON.parse(localStorage.getItem(localKey) || '[]') as Game[]; }
    catch { return []; }
  },
  async openPlatform(provider:string,section:'library'|'store'|'updates'='library'):Promise<void>{
    if(!desktopMode)throw new Error('平台入口需要 Windows 桌面版');
    return invoke('open_platform',{provider,section});
  },
  async scanGames(background = false): Promise<ScanReport> {
    if (!desktopMode) throw new Error('Steam 扫描只能在 Windows 桌面版运行');
    return invoke<ScanReport>('scan_games', {background});
  },
  async syncPlaytime(): Promise<{updated:number}> {
    return invoke('sync_playtime');
  },
  async steamAccounts(): Promise<SteamAccounts> { return desktopMode ? invoke('steam_accounts') : {accounts:[],selected:null,scope:'cache'}; },
  async selectSteamAccount(accountId:string): Promise<{updated:number}> { return invoke('select_steam_account',{accountId}); },
  async syncSteamLibrary(apiKey:string): Promise<number> { return invoke('sync_steam_library',{apiKey}); },
  async scrapeArtwork(apiKey: string, force = false, igdbClientId = '', igdbToken = ''): Promise<{processed: number; saved: number; errors: string[]}> {
    if (!desktopMode) throw new Error('刮削需要 Windows 桌面版');
    return invoke('scrape_artwork', { apiKey: apiKey.trim() || null, force, igdbClientId: igdbClientId || null, igdbToken: igdbToken || null });
  },
  async listGameScreenshots(gameId: string): Promise<string[]> {
    return desktopMode ? invoke('list_game_screenshots', {gameId}) : [];
  },
  async searchArtworkCandidates(name: string, apiKey: string, provider = 'steam'): Promise<{id:number;name:string;exact:boolean}[]> {
    return invoke('search_artwork_candidates', {name, apiKey, provider});
  },
  async confirmArtworkMatch(gameId: string, sgdbId: number, provider = 'steam'): Promise<void> {
    await invoke('confirm_artwork_match', {gameId, sgdbId, provider});
  },
  async setCustomArtwork(gameId: string, kind: 'poster'|'hero'|'landscape'|'logo'): Promise<void> {
    const path = await open({multiple:false, directory:false, filters:[{name:'图片',extensions:['png','jpg','jpeg','webp']}]});
    if (path) await invoke('set_custom_artwork', {gameId,kind,path});
  },
  async resetCustomArtwork(gameId: string, kind: 'poster'|'hero'|'landscape'|'logo'): Promise<void> {
    await invoke('reset_custom_artwork', {gameId,kind});
  },
  async addManual(): Promise<void> {
    if (!desktopMode) throw new Error('手动导入需要桌面版文件选择器');
    const path = await open({ multiple: false, directory: false, filters: [{ name: 'Windows 游戏程序', extensions: ['exe'] }] });
    if (!path) return;
    await invoke('add_manual_game', { path });
  },
  async chooseManual(folder: boolean): Promise<string | null> {
    if (!desktopMode) throw new Error('请在 Windows 桌面版导入本地游戏');
    const path = await open({ multiple: false, directory: folder, title: folder ? '选择某一款游戏的文件夹' : '选择游戏启动程序',
      ...(folder ? {} : { filters: [{ name: 'Windows 游戏程序', extensions: ['exe'] }] }) });
    return typeof path === 'string' ? path : null;
  },
  async scanManualFolder(path: string): Promise<ManualFolderScan> { return invoke('scan_manual_folder', {path}); },
  async importManual(path: string, name: string, hidden: boolean): Promise<string> { return invoke('add_manual_game', {path, name, hidden}); },
  async setHidden(gameId: string, hidden: boolean): Promise<void> {
    if (desktopMode) { await invoke('set_hidden', {gameId,hidden}); return; }
    const games = await this.listGames();
    localStorage.setItem(localKey,JSON.stringify(games.map(game=>game.id===gameId?{...game,hidden}:game)));
  },
  async setFavorite(game: Game, favorite: boolean): Promise<void> {
    if (game.id.startsWith('demo-')) return;
    if (desktopMode) { await invoke('set_favorite', { gameId: game.id, favorite }); return; }
    const list = await this.listGames();
    localStorage.setItem(localKey, JSON.stringify(list.map(g => g.id === game.id ? { ...g, favorite } : g)));
  },
  async launch(game: Game, sessionId: string, viaController = false): Promise<SessionInfo> {
    if (desktopMode && !game.installed && game.platform === 'steam') { await invoke('install_steam_game',{gameId:game.id}); return { session_id:sessionId,game_id:game.id,state:'launcher_started',timestamp:new Date().toISOString(),message:'已在 Steam 打开安装'}; }
    if(desktopMode&&!game.installed&&game.platform==='epic'){await invoke('install_epic_game',{gameId:game.id});return {session_id:sessionId,game_id:game.id,state:'launcher_started',timestamp:new Date().toISOString(),message:'已在 Epic 打开安装'};}
    if (!desktopMode || !game.launch_installation_id) throw new Error('此条目为界面演示数据，不能启动。请先扫描 Steam 或导入本地 EXE。');
    playUiSound('launch');
    return invoke<SessionInfo>('launch_game', { installationId: game.launch_installation_id, sessionId, viaController, followDisplay: localStorage.getItem('gamedeck-follow-display')!=='off' });
  },
  async subscribeSession(cb: (s: SessionInfo) => void): Promise<() => void> {
    if (!desktopMode) return () => undefined;
    return listen<SessionInfo>('game-session-changed', e => cb(e.payload));
  },
};
