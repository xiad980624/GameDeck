import type { Game } from './types';
const time=(value:string|null|undefined)=>{const n=Date.parse(value||'');return Number.isFinite(n)?n:0;};
export function homeGames(games:Game[],newIds:readonly string[]=[]):Game[]{
  const pending=new Set(newIds);
  return games.filter(game=>!game.hidden).sort((a,b)=>{
    const fresh=Number(pending.has(b.id))-Number(pending.has(a.id));if(fresh)return fresh;
    const aTime=pending.has(a.id)?time(a.added_at):Math.max(time(a.last_played_at),time(a.added_at));
    const bTime=pending.has(b.id)?time(b.added_at):Math.max(time(b.last_played_at),time(b.added_at));
    return bTime-aTime||a.name.localeCompare(b.name)||a.id.localeCompare(b.id);
  });
}
