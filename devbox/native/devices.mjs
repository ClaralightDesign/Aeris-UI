import { readFileSync } from 'node:fs';
import { join } from 'node:path';

export function loadDisplayProfiles(source) {
  return JSON.parse(readFileSync(join(source, 'crates/aeris-ui/display-profiles.json'), 'utf8'));
}

export function selectDisplayProfiles(profiles, ids) {
  if (ids === undefined) return [profiles[0]];
  if (!Array.isArray(ids) || !ids.length || new Set(ids).size !== ids.length) {
    throw new Error('DevBox 配置的 devices 需要至少一个设备 ID，且不能重复。');
  }
  return ids.map((id) => {
    const profile = profiles.find((entry) => entry.id === id);
    if (!profile) throw new Error('DevBox 配置中的设备不存在：' + id);
    return profile;
  });
}

export function embedDisplays(html, devices, isolatedStorage) {
  const data = JSON.stringify({ devices, isolatedStorage }).replaceAll('<', '\\u003c');
  return html.replace('<head>', '<head><script type="application/json" id="aeris-displays">' + data + '</script>');
}
