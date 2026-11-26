import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Search, LayoutGrid, ArrowDownWideNarrow, Heart, Gamepad2, FolderPlus, ArrowLeft, LockKeyhole } from 'lucide-react';
import { useDeck } from '../lib/store';
import { GameCard } from './GameCard';
import { Select } from './Select';
import { InputHints } from './InputHints';
import { useInputDevice } from '../lib/inputDevice';
import { backend } from '../lib/backend';
import { platformName,updatePending,libraryPlatforms } from '../lib/platforms';
import type { InputAction } from '../lib/types';
import { libraryGames, spaceGames } from '../lib/libraryScope';
import { LocalImport } from './LocalImport';
import { MoveToHidden } from './MoveToHidden';

export function Library({active=true}:{active?:boolean}) {
  const inputMode = useInputDevice(s => s.mode);
  const allGames = useDeck(s => s.games);
  const hiddenSpace = useDeck(s => s.hiddenSpace);
  const games = useMemo(()=>spaceGames(allGames,hiddenSpace),[allGames,hiddenSpace]);
  const [importOpen,setImportOpen] = useState(false);
  const [moveOpen,setMoveOpen] = useState(false);
  const focusedId = useDeck(s => s.focusedId);
  const focusSource = useDeck(s => s.focusSource);
  const setFocus = useDeck(s => s.setFocus);
  const showDetail = useDeck(s => s.showDetail);
  const query = useDeck(s => s.query);
  const setQuery = useDeck(s => s.setQuery);
  const filter = useDeck(s => s.filter);
  const setFilter = useDeck(s => s.setFilter);
  const sort = useDeck(s => s.sort);
  const platformFilter=useDeck(s=>s.platformFilter);
  const setPlatformFilter=useDeck(s=>s.setPlatformFilter);
  const platformGames=games.filter(g=>platformFilter==='all'||g.platform===platformFilter);
  const platformOptions=libraryPlatforms(games);
  const canOpen=['steam','epic','battlenet'].includes(platformFilter);
  const openPlatform=async(section:'library'|'store'|'updates')=>{try{await backend.openPlatform(platformFilter,section);}catch(error){useDeck.getState().notify(String(error));}};
  useEffect(()=>{
    const control=(event:Event)=>{
      if(!active)return;
      const target=document.activeElement as HTMLElement|null;
      if(!target?.closest('[data-library-controls]')||target.getAttribute('role')==='combobox')return;
      const action=(event as CustomEvent<InputAction>).detail;
      if(action==='CONFIRM'){target.click();event.preventDefault();}
      if(action==='LEFT'||action==='RIGHT'){
        const nodes=Array.from(document.querySelectorAll<HTMLElement>('[data-library-controls] button:not(:disabled)'));
        nodes[Math.max(0,Math.min(nodes.length-1,nodes.indexOf(target)+(action==='LEFT'?-1:1)))]?.focus();event.preventDefault();
      }
    };window.addEventListener('deck-control',control);return()=>window.removeEventListener('deck-control',control);
  },[active]);
  const setSort = useDeck(s => s.setSort);
  const demo = useDeck(s => s.demo);
  const scrollRef = useRef<HTMLDivElement>(null);
  const layoutRef = useRef<HTMLDivElement>(null);
  const [cols, setCols] = useState(6);
  const [width, setWidth] = useState(1200);
  const gap = 19;

  const filtered = useMemo(() => {
    return libraryGames({games:allGames,hiddenSpace,query,filter,sort,platformFilter});
  }, [allGames, hiddenSpace, query, filter, sort,platformFilter]);
  useEffect(() => {
    if (active && filtered.length && !filtered.some(g => g.id === focusedId)) setFocus(filtered[0].id);
  }, [active, filtered, focusedId, setFocus]);
  const measure = useCallback(() => {
    const w = layoutRef.current?.getBoundingClientRect().width || 1200;
    setWidth(w);
    setCols(Math.max(2, Math.min(10, Math.floor((w + gap) / (175 + gap)))));
  }, []);
  useEffect(() => {
    if (!layoutRef.current) return;
    const ob = new ResizeObserver(measure);
    ob.observe(layoutRef.current);
    measure();
    return () => ob.disconnect();
  }, [measure]);
  const itemWidth = (width - (cols - 1) * gap) / cols;
  const rowHeight = itemWidth * 1.5 + 36;
  const count = Math.ceil(filtered.length / cols);
  const rows = useVirtualizer({
    count, getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight + gap,
    overscan: 3,
  });
  useEffect(() => {
    if (!active || focusSource === 'pointer') return;
    const idx = filtered.findIndex(g => g.id === focusedId);
    if (idx !== -1) rows.scrollToIndex(Math.floor(idx / cols), { align: 'auto' });
    // Only scroll to new focused item, not on every remeasure.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, focusedId, focusSource, cols, filtered]);

  return <main className="library page-in">
    <div className="library-heading">
      <div><h1>{hiddenSpace&&<LockKeyhole size={26}/>} {hiddenSpace?'隐藏空间':'游戏库'}</h1>{demo && <p className="library-subtitle">演示库</p>}</div>
      <div className="library-import-actions" data-input-capture>
        {hiddenSpace&&<button className="btn btn-outline" onClick={()=>useDeck.getState().toggleHiddenSpace()}><ArrowLeft size={16}/>退出空间</button>}
        {hiddenSpace&&<button className="btn btn-outline" onClick={()=>setMoveOpen(true)}>从游戏库移入</button>}
        <button className="btn btn-outline" onClick={()=>setImportOpen(true)}><FolderPlus size={16}/>导入本地游戏</button>
      </div>
      <div className="library-counter"><strong>{String(games.length).padStart(2, '0')}</strong><span>款游戏</span></div>
    </div>
    <div className="library-platforms" data-library-controls data-input-capture onKeyDown={event=>{
      if(event.key==='Escape'){(document.activeElement as HTMLElement)?.blur();document.querySelector<HTMLElement>(`[data-game-id="${focusedId}"]`)?.focus();}
      if(['q','Q','e','E'].includes(event.key)){
        event.preventDefault();const step=event.key.toLowerCase()==='e'?1:-1;
        const next=platformOptions[(platformOptions.indexOf(platformFilter)+step+platformOptions.length)%platformOptions.length];
        setPlatformFilter(next);document.getElementById(`platform-${next}`)?.focus();
      }
    }}>

      <div className="platform-tabs" role="tablist" aria-label="游戏平台" onKeyDown={event=>{
        if(['ArrowLeft','ArrowRight','Home','End'].includes(event.key)){
          event.preventDefault();event.stopPropagation();
          const step=event.key==='ArrowRight'?1:-1;
          const index=event.key==='Home'?0:event.key==='End'?platformOptions.length-1:(platformOptions.indexOf(platformFilter)+step+platformOptions.length)%platformOptions.length;
          const next=platformOptions[index];setPlatformFilter(next);document.getElementById(`platform-${next}`)?.focus();
        }
        if(event.key==='ArrowDown'){event.preventDefault();document.querySelector<HTMLElement>(`[data-game-id="${useDeck.getState().focusedId}"]`)?.focus();}
      }}>

        {platformOptions.map(platform=><button key={platform} id={`platform-${platform}`} role="tab" aria-selected={platformFilter===platform} aria-controls="poster-wall" tabIndex={platformFilter===platform?0:-1} onClick={()=>setPlatformFilter(platform)}>{platformName(platform)}<small>{games.filter(g=>platform==='all'||g.platform===platform).length}</small></button>)}
      </div>
      <div className="platform-links"><button className="btn btn-outline" disabled={!canOpen} onClick={()=>void openPlatform('library')}>打开客户端</button><button className="btn btn-outline" disabled={!canOpen} onClick={()=>void openPlatform('store')}>浏览商店</button><button className="btn btn-outline" disabled={!canOpen} onClick={()=>void openPlatform('updates')}>检查更新</button></div>
    </div>
    <div className="library-toolbar">
      <div className="filter-tabs">
        <button className={filter === 'all' ? 'active' : ''} onClick={() => setFilter('all')}>全部 <small>{platformGames.length}</small></button>
        <button className={filter === 'installed' ? 'active' : ''} onClick={() => setFilter('installed')}>已安装 <small>{platformGames.filter(g => g.installed).length}</small></button>
        <button className={filter === 'favorites' ? 'active' : ''} onClick={() => setFilter('favorites')}><Heart size={14} /> 收藏 <small>{platformGames.filter(g => g.favorite).length}</small></button>
        <button className={filter==='updates'?'active':''} onClick={()=>setFilter('updates')}>更新 <small>{platformGames.filter(updatePending).length}</small></button>
      </div>
      <div className="library-tools">
        <div className="search-field"><Search size={18} /><input id="library-search" autoComplete="off" value={query} placeholder="搜索游戏名称 / 类型…" onChange={e => setQuery(e.target.value)} aria-label="搜索游戏" />{inputMode === 'keyboard' && <kbd>/</kbd>}</div>
        <div className="sort-wrap"><ArrowDownWideNarrow size={17} /><Select label="游戏排序" value={sort} onChange={setSort} options={[{value:'recent',label:'最近游玩'},{value:'name',label:'名称排序'},{value:'playtime',label:'游玩时间'}]}/></div>
        <div className="layout-indicator"><LayoutGrid size={20} /></div>
      </div>
    </div>
    <div className="library-main" id="poster-wall" role="tabpanel" aria-labelledby={`platform-${platformFilter}`} data-cols={cols} ref={layoutRef}>
      <div className="library-scroll" ref={scrollRef}>
        {filtered.length === 0 ? <div className="no-results"><Gamepad2 size={48} /><h3>{hiddenSpace&&games.length===0?'这里是你的隐藏空间':'这里还没有游戏'}</h3><p>{hiddenSpace?'导入游戏文件夹或 EXE，也可以从游戏库移入。':'试试其他筛选条件，或导入本地游戏。'}</p><button className="btn btn-outline" onClick={()=>setImportOpen(true)}>导入本地游戏</button></div> :
          <div style={{ height: rows.getTotalSize(), position: 'relative' }}>
            {rows.getVirtualItems().map(row => {
              const items = filtered.slice(row.index * cols, (row.index + 1) * cols);
              return <div key={row.key} className="poster-row" style={{ transform: `translateY(${row.start}px)`, gap, gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
                {items.map(game => <div className="poster-tile" key={game.id}>
                  <GameCard game={game} active={active} mode="poster" selected={focusedId === game.id} onSelect={setFocus} onOpen={showDetail} />
                  <div className="poster-caption"><strong title={game.name}>{game.name}</strong><span>{game.platform.toUpperCase()} · {game.genre.split(' · ')[0]}</span></div>
                </div>)}
              </div>;
            })}
          </div>}
      </div>
    </div>
    <footer className="library-footer"><span>{filtered.length} 款符合条件的游戏</span><InputHints context="library"/></footer>
    {importOpen&&<LocalImport hidden={hiddenSpace} onClose={()=>setImportOpen(false)}/>}
    {moveOpen&&<MoveToHidden onClose={()=>setMoveOpen(false)}/>}
  </main>;
}
