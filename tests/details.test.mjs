import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
const require = createRequire(new URL('../apps/desktop/package.json', import.meta.url));
const ts = require('typescript');
const element = (type, props) => ({ type, props });
const tree = node => !node || typeof node !== 'object' ? [] : Array.isArray(node)
  ? node.flatMap(tree) : [node, ...tree(node.props?.children)];
const text = node => node == null || typeof node === 'boolean' ? '' : typeof node !== 'object'
  ? String(node) : Array.isArray(node) ? node.map(text).join('') : text(node.props?.children);
const game = { id: 'game', name: 'Test game', description: 'Full game description', genre: 'Action', platform: 'steam', installed: true, favorite: false, total_minutes: 120, installed_version: '1.2', hero_width: 3840, hero_height: 2160 };
function fixture(overrides = {}) {
  const values = [], effects = [], calls = []; let cursor = 0;
  const state = { games: [game], detailId: game.id, hiddenSpace: false, page: 'details', libraryLoaded: true, sessions: {}, notify: () => {}, closeDetail: () => calls.push('back'), ...overrides };
  const useDeck = selector => selector(state); useDeck.getState = () => state;
  const modules = {
    'react/jsx-runtime': { jsx: element, jsxs: element, Fragment: 'fragment' },
    react: { useState: initial => { const i = cursor++; if (!(i in values)) values[i] = typeof initial === 'function' ? initial() : initial; return [values[i], value => { values[i] = typeof value === 'function' ? value(values[i]) : value; }]; }, useEffect: fn => effects.push(fn), useCallback: fn => fn, useRef: value => ({ current: value }), lazy: () => 'lazy', Suspense: 'suspense' },
    'lucide-react': { ArrowLeft: 'back-icon', Heart: 'heart', MoreHorizontal: 'more-icon' },
    'motion/react': { motion: { div: 'div' } },
    '@tauri-apps/api/core': { convertFileSrc: value => value },
    '../lib/store': { useDeck }, './lib/store': { useDeck },
    '../lib/backend': { desktopMode: false, backend: { listGameScreenshots: async id => { calls.push(['screenshots', id]); return []; } } },
    '../lib/artwork': { upgradeHero: async () => {} },
    '../lib/acceleration': { gameAccelerator: () => '', saveGameAccelerator: (id, value) => calls.push(['save', id, value]) },
    './LaunchButton': { LaunchButton: 'LaunchButton' }, './GameTitle': { GameTitle: 'GameTitle' }, './HeroImage': { HeroImage: 'HeroImage' },
    './InputHints': { InputHints: 'InputHints' }, './DeckDialog': { DeckDialog: 'DeckDialog' }, './ManualArtwork': { ManualArtwork: 'ManualArtwork' },
  };
  const load = path => { const exports = {}; vm.runInNewContext(ts.transpileModule(readFileSync(new URL(`../apps/desktop/src/${path}`, import.meta.url), 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 } }).outputText, { exports, require: name => modules[name] ?? {}, localStorage: { getItem: () => null }, ...overrides.globals }); return exports; };
  modules['../lib/platforms'] = load('lib/platforms.ts');
  const more = load('components/DetailsMore.tsx'); modules['./DetailsMore'] = more;
  return { state, modules, calls, effects, load, more, render: (fn, props) => { cursor = 0; effects.length = 0; return fn(props); } };
}

test('ordinary and hidden details expose only launch, favorite and More as main actions', () => {
  for (const hiddenSpace of [false, true]) {
    const f = fixture({ hiddenSpace }); const { Details } = f.load('components/Details.tsx');
    let favorite; const launch = () => {};
    const view = f.render(Details, { onLaunch: launch, onFavorite: value => { favorite = value; } });
    const actions = tree(view).find(n => n.props?.className === 'details-actions').props.children;
    assert.equal(actions.length, 3); assert.equal(actions[0].type, 'LaunchButton'); assert.equal(actions[0].props.onLaunch, launch);
    actions[1].props.onClick(); assert.equal(favorite, game);
    assert.equal(actions[2].props['aria-haspopup'], 'dialog');
    assert.ok(!tree(view).some(n => n.props?.className === 'detail-info-grid'));
    assert.equal(f.calls.length, 0, 'opening the page must not load screenshots or mutate settings');
  }
});

test('actionable update warnings stay visible while normal and unknown status stay in More', () => {
  for (const [status, visible] of [['unknown', false], ['not_required', false], ['not_installed', false], ['update_available', true], ['updating', true], ['paused', true], ['repair_required', true], ['installing', true]]) {
    const f = fixture({ games: [{ ...game, update_status: status }] }); const { Details } = f.load('components/Details.tsx');
    const view = f.render(Details, { onLaunch: () => {}, onFavorite: () => {} });
    assert.equal(tree(view).some(n => n.props?.role === 'status'), visible, status);
  }
});

test('More preserves saved acceleration and exposes hidden-only artwork and restore tools', () => {
  for (const hiddenSpace of [false, true]) {
    const f = fixture(); let chosen;
    const props = { game, hiddenSpace, accelerator: 'uu', onAcceleratorChange: value => { chosen = value; }, onClose: () => {} };
    let view = f.render(f.more.DetailsMore, props);
    assert.equal(tree(view).find(n => n.type === 'button' && text(n) === 'UU加速器').props['aria-pressed'], true);
    tree(view).find(n => n.type === 'button' && text(n) === '迅游加速器').props.onClick();
    assert.equal(chosen, 'xunyou'); assert.deepEqual(f.calls, [['save', game.id, 'xunyou']]);
    assert.equal(tree(view).some(n => n.type === 'button' && text(n) === '移回游戏库'), hiddenSpace);
    assert.ok(text(view).includes(game.installed_version)); assert.ok(text(view).includes(game.description));
    const images = tree(view).find(n => n.type === 'details' && text(n.props.children[0]) === '图片与背景');
    images.props.onToggle({ currentTarget: { open: true } }); view = f.render(f.more.DetailsMore, props);
    assert.equal(tree(view).some(n => n.type === 'ManualArtwork'), hiddenSpace);
    assert.ok(tree(view).some(n => n.type === 'button' && text(n) === '更新宣传背景'));
  }
});

test('the screenshot reader mounts only after its group opens', async () => {
  const f = fixture(); const props = { game, hiddenSpace: false, accelerator: '', onAcceleratorChange: () => {}, onClose: () => {} };
  let view = f.render(f.more.DetailsMore, props);
  assert.ok(!tree(view).some(n => typeof n.type === 'function'));
  const group = tree(view).find(n => n.type === 'details' && text(n.props.children[0]) === '查看游戏截图');
  group.props.onToggle({ currentTarget: { open: true } }); view = f.render(f.more.DetailsMore, props);
  const reader = tree(view).find(n => typeof n.type === 'function'); assert.ok(reader);
  f.render(reader.type, reader.props); f.effects.forEach(fn => fn());
  await Promise.resolve(); assert.deepEqual(f.calls, [['screenshots', game.id]]);
});

test('operation feedback remains readable inside the More dialog', () => {
  const f = fixture({ toast: '图片保存失败：目标文件不可写' });
  const view = f.render(f.more.DetailsMore, { game, hiddenSpace: true, accelerator: '', onAcceleratorChange: () => {}, onClose: () => {} });
  const feedback = tree(view).find(n => n.props?.role === 'status');
  assert.equal(text(feedback), f.state.toast);
});

test('the detail menu shortcut focuses and opens More instead of leaving for accelerators', () => {
  let action; const clicks = []; const moreButton = { focus: () => clicks.push('focus'), click: () => clicks.push('open') };
  const f = fixture({ globals: { window: { dispatchEvent: () => true }, document: { querySelector: selector => selector === '[data-detail-more]' ? moreButton : null, querySelectorAll: () => [], activeElement: null }, CustomEvent: class { constructor(type, init) { this.type = type; this.detail = init.detail; } } } });
  f.modules['./lib/input'] = { useDeckInput: fn => { action = fn; } };
  f.modules['./lib/inputDevice'] = { useInputDevice: { getState: () => ({ mode: 'keyboard' }) } };
  const App = f.load('App.tsx').default; f.render(App);
  action('MENU'); assert.deepEqual(clicks, ['focus', 'open']);
});
