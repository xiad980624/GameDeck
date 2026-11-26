import {GameTitle} from './GameTitle';
import { markStartup } from '../lib/startup';
import { MotionImage } from './MotionImage';
import {LaunchButton} from './LaunchButton';
import { useEffect, useMemo, useRef, useState } from 'react';
import { ArrowUpRight, Heart, Clock3, ChevronLeft, ChevronRight, MonitorPlay } from 'lucide-react';
import { useDeck } from '../lib/store';
import { GameCard } from './GameCard';
import { InputHints } from './InputHints';
import { homeGames } from '../lib/homeOrder';
import { prepareScene } from '../lib/artwork';
import type { Game } from '../lib/types';

const hrs = (minutes: number) => minutes < 60 ? minutes + ' 分钟' : (minutes / 60).toFixed(1) + ' 小时';

export function Home({ active=true,onLaunch, onFavorite }: { active?:boolean;onLaunch: (g: Game) => void; onFavorite: (g: Game) => void }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const allGames = useDeck(s => s.games);
  const games = useMemo(() => allGames.filter(game=>!game.hidden), [allGames]);
  const newGameIds=useDeck(s=>s.newGameIds);
  const focusedId = useDeck(s => s.focusedId);
  const focusSource = useDeck(s => s.focusSource);
  const setFocus = useDeck(s => s.setFocus);
  const showDetail = useDeck(s => s.showDetail);
  const setPage = useDeck(s => s.setPage);
  const [scene, setScene] = useState<Awaited<ReturnType<typeof prepareScene>> | null>(null);
  const [edges, setEdges] = useState({ left: false, right: false });
  const target = games.find(g => g.id === focusedId) ?? games[0];
  const featured = scene?.game ?? target;
  const background = scene?.background ?? '/art/fallback-hero.svg';
  const recent = useMemo(() => homeGames(games,newGameIds), [games,newGameIds]);

  // Decode all visible assets before committing a new game. Stale selections never paint.
  useEffect(() => {
    if (!active || !target) return;
    let current = true;
    void prepareScene(target).then(next => {
      if (current) { setScene(next); requestAnimationFrame(() => markStartup('hero')); }
    }).catch(() => undefined);
    return () => { current = false; };
  }, [active, target]);

  useEffect(() => {
    if (!active || !target) return;
    const index = recent.findIndex(game => game.id === target.id);
    const timer = window.setTimeout(() => {
      for (const next of [recent[index - 1], recent[index + 1]]) {
        if (next) void prepareScene(next).catch(() => undefined);
      }
    }, 80);
    return () => window.clearTimeout(timer);
  }, [active, target?.id, recent]);

  useEffect(() => {
    const strip = scrollRef.current;
    if (!strip) return;
    let frame = 0;
    let delta = 0;
    const measure = () => {
      const left=strip.scrollLeft>2,right=strip.scrollLeft+strip.clientWidth<strip.scrollWidth-2;
      setEdges(previous=>previous.left===left&&previous.right===right?previous:{left,right});
    };
    const wheel = (event: WheelEvent) => {
      if (event.ctrlKey || strip.scrollWidth <= strip.clientWidth) return;
      event.preventDefault();
      const unit = event.deltaMode === 1 ? 32 : event.deltaMode === 2 ? strip.clientWidth : 1;
      delta += (Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY) * unit;
      if (!frame) frame = requestAnimationFrame(() => {
        strip.scrollLeft += delta;
        delta = 0;
        frame = 0;
      });
    };
    strip.addEventListener('wheel', wheel, { passive: false });
    strip.addEventListener('scroll', measure, { passive: true });
    const observer = new ResizeObserver(measure);
    observer.observe(strip);
    measure();
    return () => { cancelAnimationFrame(frame); strip.removeEventListener('wheel', wheel); strip.removeEventListener('scroll', measure); observer.disconnect(); };
  }, [recent.length]);

  useEffect(() => {
    if (focusSource === 'pointer') return;
    const strip = scrollRef.current;
    const target = Array.from(strip?.querySelectorAll<HTMLElement>('[data-game-id]') ?? []).find(el => el.dataset.gameId === focusedId);
    if (!strip || !target) return;
    const card = target.getBoundingClientRect();
    const viewport = strip.getBoundingClientRect();
    if (card.left < viewport.left + 6) strip.scrollBy({ left: card.left - viewport.left - 6, behavior: 'smooth' });
    else if (card.right > viewport.right - 6) strip.scrollBy({ left: card.right - viewport.right + 6, behavior: 'smooth' });
  }, [focusedId, focusSource, recent]);

  if (!featured) return <main className="home-empty"><h1>欢迎来到 GameDeck</h1><p>在游戏库中导入你的第一款游戏。</p><button className="btn btn-primary" onClick={()=>setPage('library')}>打开游戏库</button></main>;
  return <div className="home page-in" data-featured-id={featured.id}>
    <div className="hero-backdrops" aria-hidden="true">
      <div className="hero-image-layer">
        {background && <MotionImage play={false} key={background} className="hero-image hero-image-current" src={background} alt="" draggable={false} decoding="async"/>}
      </div>
      <div className="hero-overlay" />
    </div>
    <div className="home-content">
      <div className="hero-copy">
        <GameTitle game={featured} preparedLogo={scene?.logo} className="hero-title"/>
        <div className="hero-meta"><span>{featured.genre}</span><span className="meta-separator"/><span>{featured.platform.toUpperCase()}</span>{(featured.playtime_source === 'steam-local' || featured.total_minutes > 0) && <><span className="meta-separator"/><span title={featured.playtime_source === 'steam-local' ? 'Steam 时长' : 'GameDeck 记录'}><Clock3 size={14}/> {hrs(featured.total_minutes)}</span></>}</div>
        {featured.description && <p className="hero-description">{featured.description}</p>}
        <div className="hero-actions">
          <LaunchButton game={featured} onLaunch={onLaunch}/>
          <button className="btn btn-glass" onClick={() => showDetail(featured.id)}>游戏详情 <ArrowUpRight size={19}/></button>
          <button className="btn btn-square" aria-label="收藏游戏" title="收藏游戏" onClick={() => onFavorite(featured)}><Heart size={19} fill={featured.favorite ? 'currentColor' : 'none'}/></button>
        </div>
      </div>
      <div className="home-bottom">
        <div className="section-title-row">
          <div className="home-list-title"><span className="section-badge"><MonitorPlay size={18}/></span><h2>游戏</h2></div>
          <div className="strip-tools">
            <button className="strip-arrow" aria-label="向左滚动游戏" disabled={!edges.left} onClick={() => scrollRef.current?.scrollBy({left:-Math.max(300,scrollRef.current.clientWidth*.7),behavior:'smooth'})}><ChevronLeft size={20}/></button>
            <button className="strip-arrow" aria-label="向右滚动游戏" disabled={!edges.right} onClick={() => scrollRef.current?.scrollBy({left:Math.max(300,scrollRef.current.clientWidth*.7),behavior:'smooth'})}><ChevronRight size={20}/></button>
            <button className="section-link" onClick={() => setPage('library')}>全部游戏 <ChevronRight size={18}/></button>
          </div>
        </div>
        <div className="home-strip" aria-label="游戏列表" ref={scrollRef}>
          {recent.map(game => <GameCard key={game.id} game={game} active={active} selected={focusedId === game.id} onSelect={setFocus} onOpen={showDetail} mode="wide"/>)}
        </div>
        <div className="home-footer"><span>{games.length} 款游戏</span><InputHints context="home"/></div>
      </div>
    </div>
  </div>;
}
