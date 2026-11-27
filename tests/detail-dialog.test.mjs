import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';
const require = createRequire(new URL('../apps/desktop/package.json', import.meta.url));
const ts = require('typescript');

function dialogFixture(busy = false) {
  const listeners = new Map(); let effect, closed = 0, expanded = false;
  const document = { body: {}, activeElement: null };
  const node = name => ({ name, isConnected: true, focus() { document.activeElement = this; }, matches: () => false, getClientRects: () => [1], click() {} });
  const opener = node('more'), close = node('close'), summary = node('summary'), content = node('content');
  summary.click = () => { expanded = !expanded; }; content.getClientRects = () => expanded ? [1] : [];
  document.activeElement = opener;
  const root = { querySelector: () => close, querySelectorAll: selector => selector.includes('summary') ? [close, summary, content] : [close, content] };
  const exports = {}; const element = (type, props) => ({ type, props });
  const modules = { 'react/jsx-runtime': { jsx: element, jsxs: element }, react: { useId: () => 'dialog', useRef: value => ({ current: value === null ? root : value }), useEffect: fn => { effect = fn; } }, 'react-dom': { createPortal: view => view }, 'lucide-react': { X: 'x' } };
  vm.runInNewContext(ts.transpileModule(readFileSync(new URL('../apps/desktop/src/components/DeckDialog.tsx', import.meta.url), 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }).outputText,
    { exports, require: name => modules[name], document, window: { addEventListener: (name, fn) => listeners.set(name, fn), removeEventListener: name => listeners.delete(name) } });
  exports.DeckDialog({ title: 'More', busy, onClose: () => { closed++; }, children: [] });
  const cleanup = effect();
  const send = (name, value) => { const event = { key: value, detail: value, target: document.activeElement, prevented: false, preventDefault() { this.prevented = true; }, stopPropagation() {}, stopImmediatePropagation() {} }; listeners.get(name)(event); return event; };
  return { document, send, cleanup, opener, close, summary, content, closed: () => closed };
}

test('controller can expand More sections, reach their controls and return focus on close', () => {
  const f = dialogFixture(); assert.equal(f.document.activeElement, f.close);
  assert.equal(f.send('deck-control', 'DOWN').prevented, true); assert.equal(f.document.activeElement, f.summary);
  f.send('deck-control', 'CONFIRM'); f.send('deck-control', 'DOWN'); assert.equal(f.document.activeElement, f.content);
  f.send('deck-control', 'DOWN'); assert.equal(f.document.activeElement, f.close);
  f.send('deck-control', 'BACK'); assert.equal(f.closed(), 1);
  f.cleanup(); assert.equal(f.document.activeElement, f.opener);
});

test('keyboard navigation skips collapsed content and Escape respects busy operations', () => {
  const f = dialogFixture();
  f.send('keydown', 'ArrowRight'); assert.equal(f.document.activeElement, f.summary);
  f.send('keydown', 'Tab'); assert.equal(f.document.activeElement, f.close);
  f.send('keydown', 'ArrowLeft'); assert.equal(f.document.activeElement, f.summary);
  f.send('keydown', 'Escape'); assert.equal(f.closed(), 1); f.cleanup();
  const busy = dialogFixture(true); busy.send('keydown', 'Escape'); busy.send('deck-control', 'BACK'); assert.equal(busy.closed(), 0); busy.cleanup();
});
