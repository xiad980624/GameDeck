import { isTauri } from '@tauri-apps/api/core';
import { create } from 'zustand';
import type { Game, Page, SessionInfo } from './types';
import {nextSession} from './launchState';
import { homeGames } from './homeOrder';
import { DEMO_GAMES } from './demo';
import { spaceGames } from './libraryScope';
import { playUiSound } from './uiSounds';

interface DeckState {
  sessions:Record<string,SessionInfo>;
  beginSession:(session:SessionInfo & {state:'preparing'})=>void;
  setSession:(session:SessionInfo)=>boolean;
  page: Page;
  previousPage: Page;
  hiddenSpace: boolean;
  toggleHiddenSpace: () => void;
  games: Game[];
  libraryLoaded:boolean;
  newGameIds:string[];
  acknowledgeGame:(id:string)=>void;
  demo: boolean;
  focusedId: string | null;
  focusSource: 'pointer' | 'navigation';
  detailId: string | null;
  query: string;
  filter: 'all' | 'installed' | 'favorites' | 'updates';
  platformFilter: string;
  sort: 'recent' | 'name' | 'playtime';
  accentEnabled: boolean;
  toast: string | null;
  busy: boolean;
  busyLabel: string;
  acceleratorProvider: 'uu'|'xunyou';
  acceleratorGameId: string|null;
  openAccelerators: (provider?:'uu'|'xunyou',gameId?:string|null)=>void;
  setAcceleratorProvider: (provider:'uu'|'xunyou')=>void;
  setPage: (page: Page) => void;
  setGames: (games: Game[], preserveFocus?: boolean) => void;
  setFocus: (id: string | null, source?: 'pointer' | 'navigation', audible?: boolean) => void;
  showDetail: (id: string) => void;
  closeDetail: () => void;
  setQuery: (query: string) => void;
  setFilter: (filter: DeckState['filter']) => void;
  setPlatformFilter: (platform:string)=>void;
  setSort: (sort: DeckState['sort']) => void;
  setBusy: (busy: boolean, label?: string) => void;
  notify: (text: string) => void;
  toggleFocusFx: () => void;
  updateGame: (id: string, changes: Partial<Game>) => void;
}

function storedIds(key:string):string[] {try{const value=JSON.parse(localStorage.getItem(key)||'[]');return Array.isArray(value)?value.filter((id):id is string=>typeof id==='string'):[];}catch{return [];} }
// Database freshness stamps do not change a card's visible content.
function sameVisibleGame(a:Game,b:Game){return (Object.keys(b) as (keyof Game)[]).every(key=>key==='update_checked_at'||key==='playtime_synced_at'||a[key]===b[key]);}
export const useDeck = create<DeckState>((set, get) => ({
  sessions:{},
  beginSession(session){set(state=>({sessions:{...state.sessions,[session.game_id]:session}}));},
  setSession(session){
    const previous = get().sessions[session.game_id];
    const next = nextSession(previous, session);
    if (!next || next === previous) return false;
    set(state=>({sessions:{...state.sessions,[session.game_id]:next}}));
    return true;
  },
  page: 'home', previousPage: 'home', hiddenSpace: false, games: isTauri()?[]:DEMO_GAMES, demo: !isTauri(), libraryLoaded:!isTauri(), newGameIds:storedIds('gamedeck-new-games'),
  toggleHiddenSpace() {
    if (get().page !== 'library') return;
    const hiddenSpace = !get().hiddenSpace;
    set({ hiddenSpace, query: '', filter: 'all', platformFilter: 'all', focusedId: spaceGames(get().games, hiddenSpace)[0]?.id ?? null, detailId: null, toast: null });
  },
  acknowledgeGame(id){const newGameIds=get().newGameIds.filter(value=>value!==id);localStorage.setItem('gamedeck-new-games',JSON.stringify(newGameIds));set({newGameIds});},
  focusedId: isTauri()?null:DEMO_GAMES[0].id, focusSource: 'navigation', detailId: null,
  query: '', platformFilter: 'steam', filter: 'all', sort: 'recent', accentEnabled: localStorage.getItem('gamedeck-focus-motion') !== 'false', toast: null, busy: false, busyLabel: '',
  acceleratorProvider:'uu', acceleratorGameId:null,
  openAccelerators(provider='uu',gameId=null){set({page:'accelerators',hiddenSpace:gameId?get().hiddenSpace:false,acceleratorProvider:provider,acceleratorGameId:gameId,...(gameId?{detailId:gameId,previousPage:get().page==='details'?get().previousPage:get().page}:{})});},
  setAcceleratorProvider(acceleratorProvider){set({acceleratorProvider});},
  setPage(page) { const keepSpace=(page==='details'||page==='accelerators')&&get().hiddenSpace; set({ page, hiddenSpace: keepSpace, query: '', detailId: page==='details'?get().detailId:null, toast:null, focusedId: (page==='home'?homeGames(get().games,get().newGameIds):spaceGames(get().games,keepSpace))[0]?.id ?? null }); },
  setGames(games, preserveFocus = false) {
    const actual = isTauri() || games.length > 0;
    const existing=new Map(get().games.map(game=>[game.id,game]));
    const next = (actual ? games : DEMO_GAMES).map(game=>{const cached=existing.get(game.id);return cached&&sameVisibleGame(cached,game)?cached:game;});
    const known=storedIds('gamedeck-known-games');
    const baseline=localStorage.getItem('gamedeck-known-games')!==null;
    const additions=actual&&baseline?next.filter(game=>!game.hidden&&!known.includes(game.id)).map(game=>game.id):[];
    const newGameIds=[...new Set([...get().newGameIds,...additions])];
    if(actual){localStorage.setItem('gamedeck-known-games',JSON.stringify([...new Set([...known,...next.map(game=>game.id)])]));localStorage.setItem('gamedeck-new-games',JSON.stringify(newGameIds));}
    const first=homeGames(next,newGameIds)[0]?.id??null;
    const scoped = spaceGames(next,get().hiddenSpace);
    const selectNewGame=!preserveFocus&&additions.length>0&&get().page==='home';
    const focusedId=selectNewGame?first:scoped.some(game=>game.id===get().focusedId)?get().focusedId:scoped[0]?.id??null;
    const stableGames=next.length===get().games.length&&next.every((game,index)=>game===get().games[index])?get().games:next;
    set({games:stableGames,libraryLoaded:true,demo:!actual,newGameIds,focusedId,...(selectNewGame?{focusSource:'navigation' as const}:{})});
  },
  setFocus(focusedId, focusSource = 'navigation', audible = false) {
    const changed = focusedId !== get().focusedId;
    if (changed || focusSource !== get().focusSource) set({ focusedId, focusSource });
    if (audible && changed && focusedId) playUiSound('select');
  },
  showDetail(id) { if(!spaceGames(get().games,get().hiddenSpace).some(game=>game.id===id))return; get().acknowledgeGame(id); set({ previousPage: get().page, page: 'details', detailId: id, focusedId: id }); },
  closeDetail() { set({ page: get().previousPage, detailId: null }); },
  setQuery(query) { set({ query }); },
  setFilter(filter) { set({ filter }); },
  setPlatformFilter(platformFilter){set({platformFilter,focusedId:null});},
  setSort(sort) { set({ sort }); },
  setBusy(busy, label = '') { set({ busy, busyLabel: busy ? label : '' }); },
  notify(toast) { set({ toast }); },
  toggleFocusFx() { const next = !get().accentEnabled; localStorage.setItem('gamedeck-focus-motion', String(next)); set({ accentEnabled: next }); },
  updateGame(id, changes) {
    const previous=get().games;const games=previous.map(game=>{if(game.id!==id)return game;const next={...game,...changes};return sameVisibleGame(game,next)?game:next;});
    if(games.some((game,index)=>game!==previous[index]))set({games});
  },
}));
