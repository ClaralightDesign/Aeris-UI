import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import ts from 'typescript';

const { outputText } = ts.transpileModule(readFileSync(new URL('./web/node-properties.ts', import.meta.url), 'utf8'),
  { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } });
const { styleProperties, textProperties } = await import('data:text/javascript;base64,' + Buffer.from(outputText).toString('base64'));
const label = {
  x: 26, y: 139, width: 106, height: 32, contentX: 26, contentY: 139, contentWidth: 106, contentHeight: 32,
  margin: [0, 0, 0, 0], borderWidths: [0, 0, 0, 0],
  kind: 1, foreground: 0xaaaaaa, background: 0xffffff, backgroundOpacity: 0, opacity: 255,
  fontHeight: 28, fontSize: 22, renderedFontSize: 22, renderedLineHeight: 28, fontApx: 4,
  fontFamily: 'Linoval Sans', fontWeight: 450, alignment: 1, radius: 0, border: 0,
};
const values = (rows) => Object.fromEntries(rows.map(({ label, value }) => [label, value]));

test('apx, rendered font size and line height remain distinct', () => {
  const rows = values(textProperties(label));
  assert.equal(rows['字号'], '4 apx → 22 px');
  assert.equal(rows['实际渲染字号'], '22 px');
  assert.equal(rows['文字行高'], '28 px');
  assert.equal(rows['文字颜色'], '#aaaaaa');
});

test('scroll scaling reports the renderer result without replacing the source token', () => {
  const rows = values(textProperties({ ...label, fontApx: 7, fontSize: 39,
    renderedFontSize: 31.96875, renderedLineHeight: 38.5078125 }));
  assert.equal(rows['字号'], '7 apx → 39 px');
  assert.equal(rows['实际渲染字号'], '31.97 px');
  assert.equal(rows['文字行高'], '38.51 px');
});

test('tooltip omits zero-valued styles and preserves black text', () => {
  const rows = values(styleProperties({ ...label, foreground: 0 }).filter((row) => !row.zero));
  assert.equal(rows['文字颜色'], '#000000');
  for (const key of ['背景颜色', '背景透明度', '圆角', '边框宽度', '外边距', '边框', '内边距']) assert.ok(!(key in rows));
  assert.equal(rows['整体透明度'], '100%');
});

test('containers do not report an inherited default line height as typography', () => {
  assert.deepEqual(textProperties({ ...label, kind: 3 }), []);
});
