import { convertFileSrc } from '@tauri-apps/api/core';
import { backend, desktopMode } from './backend';
import { useDeck } from './store';
import { getHero } from './demo';
import type { Game } from './types';

const displayPaths=new Map<string,string>();
export function primeHeroPaths(paths:Record<string,string>){for(const [source,path] of Object.entries(paths))displayPaths.set(source,path);}
const heroes = new Map<string, Promise<HTMLImageElement>>();
const readyHeroes = new Map<string, HTMLImageElement>();
const heroKey=(game:Game)=>desktopMode&&!game.hero?'/art/fallback-hero.svg':getHero(game);
export const artworkUrl=(source:string)=>desktopMode&&/^[A-Za-z]:[\\/]/.test(source)?convertFileSrc(source):source;
const titles=new Map<string,Promise<HTMLImageElement|null>>();
export function prepareTitle(game:Game):Promise<HTMLImageElement|null>{
  if(!game.logo)return Promise.resolve(null);
  const source=game.logo;const cached=titles.get(source);if(cached)return cached;
  const task=(async()=>{const img=new Image();img.decoding='async';img.src=artworkUrl(source);try{await img.decode();return img;}catch{titles.delete(source);return null;}})();
  titles.set(source,task);return task;
}
export function peekHero(game:Game):HTMLImageElement|undefined{return readyHeroes.get(heroKey(game));}
export function prepareHero(game: Game): Promise<HTMLImageElement> {
  const source = desktopMode&&!game.hero?'/art/fallback-hero.svg':heroKey(game);
  const url = desktopMode && /^[A-Za-z]:[\\/]/.test(source) ? convertFileSrc(source) : source;
  const cached = heroes.get(source);
  if (cached) return cached;
  const task = (async () => {
    const img = new Image();
    img.decoding = 'async';
    img.fetchPriority = 'high';
    let display=url;
    if(desktopMode&&game.hero){try{const path=displayPaths.get(game.hero)??await backend.heroDisplayPath(game.id);if(path){displayPaths.set(game.hero,path);display=convertFileSrc(path);}}catch{/* Keep the existing original as a fallback. */}}
    img.src = display;
    try { await img.decode(); }
    catch { heroes.delete(source);img.src = '/art/fallback-hero.svg'; await img.decode(); }
    readyHeroes.set(source,img);
    while(readyHeroes.size>6){const oldest=readyHeroes.keys().next().value!;readyHeroes.delete(oldest);heroes.delete(oldest);}
    return img;
  })();
  heroes.set(source, task);
  return task;
}
export async function prepareScene(game:Game){const [hero,logo]=await Promise.all([prepareHero(game),prepareTitle(game)]);return {game,background:hero.src,logo:logo?.src??null};}

const attemptedUpgrades=new Set<string>();
const upgrades=new Map<string,Promise<void>>();
export function upgradeHero(game:Game,retry=false):Promise<void>{
  const pending=upgrades.get(game.id);if(pending)return pending;
  if(!retry&&attemptedUpgrades.has(game.id))return Promise.resolve();
  attemptedUpgrades.add(game.id);
  const task=backend.improveBackground(game.id).then(games=>{useDeck.getState().setGames(games);}).finally(()=>upgrades.delete(game.id));
  upgrades.set(game.id,task);return task;
}
