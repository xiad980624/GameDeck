import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';
const require = createRequire(new URL('../apps/desktop/package.json', import.meta.url));
const ts = require('typescript');

function fixture({ native = true, stored = null, reply = () => Promise.resolve(true), blocked = false } = {}) {
  const calls = [], players = [], memory = new Map(stored ? [['gamedeck-ui-sounds', stored]] : []);
  const document = { hidden: false };
  let now = 0;
  const cache = new Map();
  const storage = { getItem: key => memory.get(key) ?? null, setItem: (key, value) => memory.set(key, value) };
  class Audio {
    constructor(src) { this.src = src; this.currentTime = 0; this.paused = true; this.plays = 0; players.push(this); }
    pause() { this.paused = true; }
    play() { this.paused = false; this.plays++; return blocked ? Promise.reject(Error('autoplay denied')) : Promise.resolve(); }
  }
  function load(name) {
    if (cache.has(name)) return cache.get(name);
    const exports = {}; cache.set(name, exports);
    const customRequire = id => id === '@tauri-apps/api/core'
      ? { isTauri: () => native, invoke: (command, args) => { calls.push({ command, ...args }); return reply(); } }
      : id.startsWith('./') ? load(id.slice(2)) : require(id);
    vm.runInNewContext(ts.transpileModule(readFileSync(new URL(`../apps/desktop/src/lib/${name}.ts`, import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText,
      { exports, require: customRequire, localStorage: storage, performance: { now: () => now }, document, Audio });
    return exports;
  }
  return { audio: load('uiSounds'), load, calls, players, memory, document, advance: ms => { now += ms; } };
}

test('native controller cues work with stale WebView visibility and limit rapid selections', () => {
  const f = fixture(); f.document.hidden = true;
  f.audio.playUiSound('select'); f.audio.playUiSound('select');
  assert.deepEqual(f.calls.map(c => c.sound), ['select']);
  f.advance(75); f.audio.playUiSound('select'); f.audio.playUiSound('launch');
  f.advance(100); f.audio.playUiSound('select');
  assert.deepEqual(f.calls.map(c => c.sound), ['select', 'select', 'launch']);
  f.advance(250); f.audio.playUiSound('select'); assert.equal(f.calls.length, 4);
  assert.equal(f.players.length, 0);
});

test('mute stops playback immediately, persists, and is respected after a reload', () => {
  const f = fixture(); f.audio.playUiSound('launch'); f.audio.setUiSoundsEnabled(false);
  f.audio.playUiSound('launch'); f.advance(1000); f.audio.playUiSound('select');
  assert.deepEqual(f.calls.map(c => c.sound), ['launch', null]);
  assert.equal(f.memory.get('gamedeck-ui-sounds'), 'off');
  const reloaded = fixture({ stored: f.memory.get('gamedeck-ui-sounds') });
  assert.equal(reloaded.audio.uiSoundsEnabled(), false);
  reloaded.audio.playUiSound('launch'); assert.equal(reloaded.calls.length, 0);
  reloaded.audio.setUiSoundsEnabled(true); reloaded.audio.playUiSound('select');
  assert.equal(reloaded.calls[0].sound, 'select');
});

test('browser cues reuse two players, stop the prior cue, and ignore hidden pages', async () => {
  const f = fixture({ native: false, blocked: true });
  f.audio.playUiSound('select'); f.audio.playUiSound('launch');
  assert.equal(f.players[0].paused, true); assert.equal(f.players[1].paused, false);
  f.advance(350); f.audio.playUiSound('select');
  assert.equal(f.players.length, 2); assert.equal(f.players[0].plays, 2); assert.equal(f.players[1].paused, true);
  f.document.hidden = true; f.audio.playUiSound('launch'); assert.equal(f.players[1].plays, 1);
  f.audio.setUiSoundsEnabled(false); assert.ok(f.players.every(p => p.paused));
  await new Promise(resolve => setImmediate(resolve)); // Rejected play promises are contained.
});

test('late unsupported-platform replies cannot replay a cue after mute or a newer request', async () => {
  const pending = []; const f = fixture({ reply: () => new Promise(resolve => pending.push(resolve)) });
  f.audio.playUiSound('select'); f.audio.playUiSound('launch');
  pending[0](false); await Promise.resolve(); assert.equal(f.players.length, 0);
  f.audio.setUiSoundsEnabled(false); pending[1](false); await Promise.resolve();
  assert.equal(f.players.length, 0);
});

test('audio transport failures never escape to navigation or launch callers', async () => {
  for (const reply of [() => { throw Error('IPC unavailable'); }, () => Promise.reject(Error('IPC unavailable'))]) {
    const f = fixture({ reply });
    assert.doesNotThrow(() => f.audio.playUiSound('launch'));
    assert.doesNotThrow(() => f.audio.setUiSoundsEnabled(false));
  }
  await new Promise(resolve => setImmediate(resolve));
});

test('only explicit selection changes sound; refresh, focus repair and duplicate focus stay silent', () => {
  const f = fixture(); const { useDeck } = f.load('store');
  const game = id => ({ id, name: id, original_name: id, hidden: false, installed: true, platform: 'manual', added_at: '2026-11-26', last_played_at: null });
  useDeck.getState().setGames([game('a'), game('b')]);
  useDeck.getState().setFocus('b'); assert.equal(f.calls.length, 0);
  useDeck.getState().setFocus('a', 'navigation', true);
  useDeck.getState().setFocus('a', 'pointer', true); assert.equal(f.calls.length, 1);
  f.advance(100); useDeck.getState().setGames([game('b')]);
  useDeck.getState().setFocus(null, 'navigation', true); assert.equal(f.calls.length, 1);
  useDeck.getState().setFocus('b', 'pointer', true); assert.equal(f.calls.length, 2);
});

test('bundled cues are short quiet PCM audio with silent endpoints', () => {
  for (const [name, duration] of [['select', 0.065], ['launch', 0.3]]) {
    const wav = readFileSync(new URL(`../apps/desktop/public/audio/${name}.wav`, import.meta.url));
    assert.equal(wav.toString('ascii', 0, 4), 'RIFF'); assert.equal(wav.toString('ascii', 8, 12), 'WAVE');
    assert.equal(wav.readUInt16LE(20), 1); assert.equal(wav.readUInt16LE(22), 1);
    assert.equal(wav.readUInt32LE(24), 44100); assert.equal(wav.readUInt16LE(34), 16);
    assert.ok(Math.abs((wav.length - 44) / 2 / 44100 - duration) < 0.0001);
    assert.equal(wav.readInt16LE(44), 0); assert.equal(wav.readInt16LE(wav.length - 2), 0);
    let peak = 0; for (let i = 44; i < wav.length; i += 2) peak = Math.max(peak, Math.abs(wav.readInt16LE(i)));
    assert.ok(peak > 1000 && peak < 10000);
  }
});
