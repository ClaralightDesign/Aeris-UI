import hljs from 'highlight.js/lib/core';
import bash from 'highlight.js/lib/languages/bash';
import cpp from 'highlight.js/lib/languages/cpp';
import css from 'highlight.js/lib/languages/css';
import ini from 'highlight.js/lib/languages/ini';
import javascript from 'highlight.js/lib/languages/javascript';
import json from 'highlight.js/lib/languages/json';
import makefile from 'highlight.js/lib/languages/makefile';
import markdown from 'highlight.js/lib/languages/markdown';
import python from 'highlight.js/lib/languages/python';
import rust from 'highlight.js/lib/languages/rust';
import typescript from 'highlight.js/lib/languages/typescript';
import xml from 'highlight.js/lib/languages/xml';
import yaml from 'highlight.js/lib/languages/yaml';

for (const [name, grammar] of Object.entries({
  bash, cpp, css, ini, javascript, json, makefile, markdown, python, rust, typescript, xml, yaml,
})) {
  hljs.registerLanguage(name, grammar);
}

const languages: Record<string, string> = {
  c: 'cpp', h: 'cpp', cc: 'cpp', cpp: 'cpp', hpp: 'cpp', css: 'css', html: 'xml', svg: 'xml',
  js: 'javascript', jsx: 'javascript', mjs: 'javascript', cjs: 'javascript', json: 'json',
  md: 'markdown', mdx: 'markdown', py: 'python', rs: 'rust', sh: 'bash', toml: 'ini',
  ts: 'typescript', tsx: 'typescript', yaml: 'yaml', yml: 'yaml',
};

export function highlight(path: string, text: string) {
  const ext = path.split('.').at(-1)?.toLowerCase() ?? '';
  const language = path.split('/').at(-1) === 'Makefile' ? 'makefile' : languages[ext];
  return language && hljs.getLanguage(language) && text.length <= 200_000
    ? hljs.highlight(text, { language, ignoreIllegals: true }).value
    : undefined;
}
