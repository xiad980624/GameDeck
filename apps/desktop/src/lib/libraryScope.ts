import type { Game } from './types';
import { updatePending } from './platforms';

export function spaceGames(games: Game[], hiddenSpace = false): Game[] {
  return games.filter(game => Boolean(game.hidden) === hiddenSpace);
}

export function libraryGames(state: { games: Game[]; hiddenSpace: boolean; query: string; platformFilter: string;
  filter: 'all' | 'installed' | 'favorites' | 'updates'; sort: 'recent' | 'name' | 'playtime' }): Game[] {
  const q = state.query.trim().toLowerCase();
  return spaceGames(state.games, state.hiddenSpace).filter(game =>
    (!q || `${game.name} ${game.original_name || ''} ${game.genre} ${game.platform}`.toLowerCase().includes(q)) &&
    (state.platformFilter === 'all' || game.platform === state.platformFilter) &&
    (state.filter === 'all' || (state.filter === 'installed' && game.installed) ||
      (state.filter === 'favorites' && game.favorite) || (state.filter === 'updates' && updatePending(game))))
    .sort((a, b) => state.sort === 'name' ? a.name.localeCompare(b.name) : state.sort === 'playtime' ?
      b.total_minutes - a.total_minutes : (b.last_played_at || b.added_at).localeCompare(a.last_played_at || a.added_at));
}

export function hiddenShortcut(event: Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey' | 'repeat' | 'isComposing'>,
  page: string, editing: boolean): boolean {
  return page === 'library' && !editing && !event.repeat && !event.isComposing && event.code === 'KeyH' &&
    event.ctrlKey && event.shiftKey && !event.altKey && !event.metaKey;
}
