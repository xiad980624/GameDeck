import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import test from 'node:test';

const require = createRequire(new URL('../apps/desktop/package.json', import.meta.url));
const ts = require('typescript');
const source = readFileSync(new URL('../apps/desktop/src/lib/inputPrompts.ts', import.meta.url), 'utf8');
const output = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
const exports = {};
vm.runInNewContext(output, { exports });
const { detectInputDevice, inputLabels, keyboardDevice } = exports;
const pad = (id, axes=[0,0], active=false) => ({ id, connected:true, axes, buttons:[{pressed:active,value:active?1:0}] });

test('an idle connected controller does not steal keyboard hints', () => {
  assert.equal(detectInputDevice(keyboardDevice,[pad('Xbox Wireless Controller')]).mode,'keyboard');
});
test('an actual Xbox press switches to ABXY rather than PlayStation symbols', () => {
  const input = detectInputDevice(keyboardDevice,[pad('Xbox Wireless Controller',[0,0],true)]);
  assert.equal(input.family,'xbox');
  assert.equal(inputLabels(input).confirm,'A');
  assert.equal(inputLabels(input).prev,'LB');
});
test('Sony hardware switches all action labels together', () => {
  const input = detectInputDevice(keyboardDevice,[pad('Wireless Controller Vendor:054c',[0.8,0])]);
  const labels = inputLabels(input);
  assert.equal(input.family,'playstation');
  assert.equal(labels.confirm,'×');
  assert.equal(labels.back,'○');
  assert.equal(labels.favorite,'□');
  assert.equal(labels.menu,'△');
  assert.equal(labels.prev,'L1');
});
test('stick noise stays below the activation threshold', () => {
  assert.equal(detectInputDevice(keyboardDevice,[pad('Xbox',[0.2,-0.3])]).mode,'keyboard');
});
test('releasing a button keeps controller hints until another device is used', () => {
  const input = detectInputDevice(keyboardDevice,[pad('Xbox',[0,0],true)]);
  assert.equal(detectInputDevice(input,[pad('Xbox')]).mode,'gamepad');
  assert.equal(inputLabels(keyboardDevice).confirm,'Enter');
});
test('disconnecting the controller restores keyboard hints', () => {
  assert.equal(detectInputDevice({mode:'gamepad',family:'xbox'},[null]).mode,'keyboard');
});
