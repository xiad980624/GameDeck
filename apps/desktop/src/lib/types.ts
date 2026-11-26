export type Page = 'home' | 'library' | 'details' | 'settings' | 'accelerators';
export type Source = 'steam' | 'manual' | 'battlenet' | 'epic' | 'kuro';

export interface Game {
  id: string;
  name: string;
  original_name?: string;
  update_status?: 'unknown'|'not_installed'|'not_required'|'update_available'|'updating'|'paused'|'repair_required'|'installing';
  installed_version?: string|null;
  update_detail?: string|null;
  update_checked_at?: string|null;
  description: string;
  genre: string;
  platform: Source | string;
  external_id: string;
  launch_installation_id: string | null;
  installed: boolean;
  favorite: boolean;
  hidden?: boolean;
  total_minutes: number;
  last_played_at: string | null;
  added_at: string;
  poster: string | null;
  hero: string | null;
  logo?:string|null;
  hero_width?:number|null;
  hero_height?:number|null;
  hero_source?:string|null;
  landscape?: string | null;
  local_minutes?: number;
  playtime_source?: 'steam-local' | 'gamedeck';
  playtime_synced_at?: string | null;
  accent?: string;
  tagline?: string;
}

export type ScanReport = { found: number; inserted: number; updated: number; errors: string[] };
export type SessionState = 'preparing' | 'launcher_started' | 'awaiting_game' | 'running' | 'exited' | 'failed';
export interface SessionInfo {
  session_id: string;
  game_id: string;
  state: SessionState;
  timestamp: string;
  message?: string;
}
export type InputAction = 'UP' | 'DOWN' | 'LEFT' | 'RIGHT' | 'CONFIRM' | 'BACK' | 'MENU' | 'SEARCH' | 'FAVORITE' | 'PAGE_PREV' | 'PAGE_NEXT';
