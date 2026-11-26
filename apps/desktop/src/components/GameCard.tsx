import {useInputDevice} from '../lib/inputDevice';
import {realPointerMove} from '../lib/controller';
import { Heart, Play, Check } from 'lucide-react';
import { memo, useEffect } from 'react';
import { prepareHero } from '../lib/artwork';
import type { Game } from '../lib/types';
import { useDeck } from '../lib/store';
import { Artwork } from './Artwork';
import { updateLabel,updatePending } from '../lib/platforms';

export const GameCard = memo(function GameCard({ game, selected, onSelect, onOpen, mode = 'wide', active=true }: {
  game: Game; active?:boolean; selected: boolean; onSelect: (id: string, source?: 'pointer' | 'navigation', audible?: boolean) => void; onOpen: (id: string) => void; mode?: 'wide' | 'poster';
}) {
  const isNew=useDeck(s=>s.newGameIds.includes(game.id));
  useEffect(()=>{if(!active||!selected)return;const timer=window.setTimeout(()=>{void prepareHero(game).catch(()=>undefined);},120);return()=>window.clearTimeout(timer);},[active,selected,game.id,game.hero]);
  return <button
    type="button"
    className={`game-card ${mode} ${selected ? 'is-selected' : ''}`}
    onPointerMove={e => { if (e.pointerType === 'mouse'&&realPointerMove(e)&&useInputDevice.getState().mode!=='gamepad') onSelect(game.id, 'pointer', true); }} onFocus={() => onSelect(game.id, 'navigation', true)} onClick={() => onOpen(game.id)}
    aria-label={`打开 ${game.name} 详情`}
    tabIndex={selected ? 0 : -1}
    data-game-id={game.id}
    onPointerDown={()=>{void prepareHero(game).catch(()=>undefined);}}
  >
    <Artwork play={active&&selected} game={game} kind={mode === 'wide' ? 'landscape' : 'poster'} loading="lazy" className="game-card-image" />
    <span className="game-card-shade" />
    {isNew&&<span className="card-new" aria-label="新增游戏">新增</span>}
    {updatePending(game)&&<span className="card-update">{updateLabel(game)}</span>}
    {mode === 'wide' ? <span className="game-card-name">{game.name}</span> : null}
    {game.favorite ? <span className="card-heart"><Heart size={14} fill="currentColor" /></span> : null}
    {game.installed ? <span className="card-installed"><Check size={12} /> 已安装</span> : null}
    {selected ? <span className="card-selected-icon"><Play size={15} fill="currentColor" /></span> : null}
  </button>;
});
