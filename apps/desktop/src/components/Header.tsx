import { Compass, Grid3X3, Settings2, Zap, Gamepad2, Circle } from 'lucide-react';
import { useDeck } from '../lib/store';
import { desktopMode } from '../lib/backend';
import { WindowControls } from './WindowControls';
import {useWindowState} from '../lib/window';

export function Header() {
  const fullscreen=useWindowState(s=>s.fullscreen);
  const page = useDeck(s => s.page);
  const setPage = useDeck(s => s.setPage);
  return <header className={`header ${desktopMode ? `header-native ${fullscreen?'header-fullscreen':''}` : ''}`} data-tauri-drag-region>
    <button className="brand" onClick={() => setPage('home')} aria-label="GameDeck 首页">
      <span className="brand-mark"><Gamepad2 size={25} strokeWidth={2.3} /></span>
      <span>GAME<span className="brand-alt">DECK</span><small>DESKTOP</small></span>
    </button>
    <nav className="primary-nav" aria-label="主导航">
      <button onClick={() => setPage('home')} className={`nav-item ${page === 'home' ? 'current' : ''}`}><Compass size={18} /> 大厅</button>
      <button onClick={() => setPage('library')} className={`nav-item ${page === 'library' || page === 'details' ? 'current' : ''}`}><Grid3X3 size={18} /> 游戏库</button>
      <button onClick={() => useDeck.getState().openAccelerators()} className={`nav-item ${page==='accelerators'?'current':''}`}><Zap size={18} /> 加速器</button>
    </nav>
    <div className="header-right">
      <span className="header-platform"><Circle size={8} fill="#84eeaa" color="#84eeaa" /> {desktopMode ? 'WINDOWS DESKTOP' : 'WEB PREVIEW'}</span>
      <button className={`icon-button ${page === 'settings' ? 'icon-active' : ''}`} onClick={() => setPage('settings')} title="设置" aria-label="打开设置"><Settings2 size={22} /></button>
    </div>
    {desktopMode && <WindowControls />}
  </header>;
}
