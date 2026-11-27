import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { DeckDialog } from './DeckDialog';
import { ManualArtwork } from './ManualArtwork';
import { backend, desktopMode } from '../lib/backend';
import { upgradeHero } from '../lib/artwork';
import { saveGameAccelerator, type AcceleratorId } from '../lib/acceleration';
import { platformName, updateLabel } from '../lib/platforms';
import { useDeck } from '../lib/store';
import type { Game } from '../lib/types';

export const gamePlaytime = (game: Game) => game.total_minutes || game.playtime_source === 'steam-local'
  ? `${(game.total_minutes / 60).toFixed(1)} 小时` : '暂无游玩记录';

function GameScreenshots({ game }: { game: Game }) {
  const [paths, setPaths] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    void backend.listGameScreenshots(game.id).then(value => { if (active) setPaths(value); })
      .catch(reason => { if (active) setError(String(reason)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [game.id]);
  if (loading) return <p role="status">正在读取截图…</p>;
  if (error) return <p role="alert">截图读取失败：{error}</p>;
  if (!paths.length) return <p>暂无游戏截图。</p>;
  return <div className="details-screenshots">{paths.map((path, index) =>
    <img key={path} loading="lazy" decoding="async" src={desktopMode ? convertFileSrc(path) : path} alt={`${game.name} 截图 ${index + 1}`} />
  )}</div>;
}

export function DetailsMore({ game, hiddenSpace, accelerator, onAcceleratorChange, onClose }: {
  game: Game; hiddenSpace: boolean; accelerator: AcceleratorId | '';
  onAcceleratorChange: (value: AcceleratorId | '') => void; onClose: () => void;
}) {
  const [moving, setMoving] = useState(false);
  const [upgrading, setUpgrading] = useState(false);
  const [screenshotsOpen, setScreenshotsOpen] = useState(false);
  const [artworkOpen, setArtworkOpen] = useState(false);
  const notify = useDeck(s => s.notify);
  const notice = useDeck(s => s.toast);
  const chooseAccelerator = (value: AcceleratorId | '') => {
    try { saveGameAccelerator(game.id, value); onAcceleratorChange(value); }
    catch (error) { notify(`加速设置保存失败：${String(error)}`); }
  };
  const restore = async () => {
    setMoving(true);
    try {
      await backend.setHidden(game.id, false);
      useDeck.getState().updateGame(game.id, { hidden: false });
      useDeck.getState().closeDetail();
      notify('已移回普通游戏库');
    } catch (error) { notify(String(error)); }
    finally { setMoving(false); }
  };
  const refreshBackground = async () => {
    setUpgrading(true);
    try { await upgradeHero(game, true); notify('已按宣传主视觉规则更新背景'); }
    catch (error) { notify(String(error)); }
    finally { setUpgrading(false); }
  };
  const source = game.hero_source?.startsWith('background-') ? '背景专用素材'
    : ({ 'steam-local': 'Steam 本地缓存', 'epic-local': 'Epic 本地缓存', 'battlenet-local': '战网本地缓存' }[game.hero_source || ''] || '导入 / 刮削素材');
  return <DeckDialog title="更多选项" onClose={onClose} busy={moving}>
    <p className="details-more-name">{game.name}</p>
    {notice && <p className="details-more-feedback" role="status">{notice}</p>}
    <section className="details-more-accelerator" aria-label="启动前加速">
      <h3>启动前加速</h3>
      <div className="setting-buttons">{([{ value: '', label: '不使用' }, { value: 'uu', label: 'UU加速器' }, { value: 'xunyou', label: '迅游加速器' }] as const).map(option =>
        <button key={option.value} className="btn btn-outline" aria-pressed={accelerator === option.value} onClick={() => chooseAccelerator(option.value)}>{option.label}</button>
      )}</div>
      <button className="btn btn-text" onClick={() => useDeck.getState().openAccelerators(accelerator || 'uu', game.id)}>打开加速器页</button>
    </section>
    {['steam', 'epic', 'battlenet'].includes(game.platform) && <div className="details-more-actions">
      <button className="btn btn-outline" onClick={() => void backend.openPlatform(game.platform, 'updates').catch(error => notify(String(error)))}>打开客户端检查更新</button>
    </div>}
    <details className="details-more-section">
      <summary>游戏信息与完整简介</summary>
      {game.description && <p>{game.description}</p>}
      <dl className="details-more-info">
        <dt>游戏平台</dt><dd>{platformName(game.platform)}</dd>
        <dt>安装状态</dt><dd>{game.installed ? '已安装' : '未安装'}</dd>
        <dt>{game.playtime_source === 'steam-local' ? 'Steam 时长' : 'GameDeck 记录'}</dt><dd>{gamePlaytime(game)}</dd>
        <dt>更新状态</dt><dd>{updateLabel(game)}{game.update_detail && <p>{game.update_detail}</p>}</dd>
        {game.installed_version && <><dt>已安装版本</dt><dd>{game.installed_version}</dd></>}
      </dl>
    </details>
    <details className="details-more-section" onToggle={event => setArtworkOpen(event.currentTarget.open)}>
      <summary>图片与背景</summary>
      {!!game.hero_width && !!game.hero_height && <p>背景原图 {game.hero_width} × {game.hero_height} · {source}</p>}
      <div className="details-more-actions"><button className="btn btn-outline" disabled={upgrading} onClick={() => void refreshBackground()}>{upgrading ? '正在获取原图…' : '更新宣传背景'}</button></div>
      {hiddenSpace && artworkOpen && <ManualArtwork key={game.id} gameId={game.id} />}
    </details>
    <details className="details-more-section" onToggle={event => setScreenshotsOpen(event.currentTarget.open)}>
      <summary>查看游戏截图</summary>
      {screenshotsOpen && <GameScreenshots key={game.id} game={game} />}
    </details>
    {hiddenSpace && <div className="details-more-actions"><button className="btn btn-outline" disabled={moving} onClick={() => void restore()}>{moving ? '正在移回…' : '移回游戏库'}</button></div>}
  </DeckDialog>;
}
