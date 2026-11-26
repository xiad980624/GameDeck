import { MotionImage } from './MotionImage';
import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { desktopMode } from '../lib/backend';
import type { Game } from '../lib/types';
import { getPoster, getHero, getLandscape, getFallbackPoster } from '../lib/demo';

export function Artwork({ game, kind, className = '', loading, play = true }: { game: Game; kind: 'poster' | 'hero' | 'landscape'; className?: string; loading?: 'lazy' | 'eager'; play?:boolean }) {
  const source = kind === 'poster' ? getPoster(game) : kind === 'landscape' ? getLandscape(game) : getHero(game);
  const resolved = desktopMode && source && /^[A-Za-z]:[\\/]/.test(source) ? convertFileSrc(source) : source;
  const fallback = kind === 'poster' ? getFallbackPoster(game) : '/art/fallback-hero.svg';
  const [failed, setFailed] = useState<string | null>(null);
  const src = failed === resolved ? fallback : resolved;
  return <MotionImage key={game.id + ':' + src} maxStillWidth={kind==='hero'?2560:640} play={kind==='hero'?false:play} className={className} src={src} alt={game.name} draggable={false} decoding="async" loading={loading ?? (kind === 'poster' ? 'lazy' : 'eager')} onError={() => setFailed(resolved)} />;
}
