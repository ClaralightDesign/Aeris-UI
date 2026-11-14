import { readFile, realpath, stat } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import sharp from 'sharp';
import { parse } from 'smol-toml';

const size = 128;
const iconSize = 108;
const inset = (size - iconSize) / 2;
const limit = 16 * 1024 * 1024;
const circle = Buffer.from('<svg width="108" height="108" xmlns="http://www.w3.org/2000/svg"><circle cx="54" cy="54" r="54" fill="white"/></svg>');

// Canopus can also declare an uncompressed LVGL 9 launcher bitmap.
function lvglImage(bytes) {
  if (bytes.length < 12 || bytes.readUInt32LE(0) !== 0x1019) throw new Error('Unsupported LVGL icon');
  const width = bytes.readUInt16LE(4);
  const height = bytes.readUInt16LE(6);
  const stride = bytes.readUInt16LE(8);
  if (!width || !height || stride !== width * 4 || bytes.length !== 12 + stride * height) {
    throw new Error('Invalid LVGL icon dimensions');
  }
  const pixels = bytes.subarray(12);
  for (let offset = 0; offset < pixels.length; offset += 4) {
    const blue = pixels[offset];
    pixels[offset] = pixels[offset + 2];
    pixels[offset + 2] = blue;
  }
  return sharp(pixels, { raw: { width, height, channels: 4 } });
}

async function compose(icon, frame) {
  const square = await icon.rotate().resize(iconSize, iconSize, { fit: 'cover' }).ensureAlpha().png().toBuffer();
  const rounded = await sharp(square).composite([{ input: circle, blend: 'dest-in' }]).png().toBuffer();
  return sharp({ create: { width: size, height: size, channels: 4, background: '#00000000' } })
    .composite([{ input: rounded, top: inset, left: inset }, { input: frame, top: 0, left: 0 }])
    .png().toBuffer();
}

export function createFavicon(assets, project, demo) {
  const frame = join(assets, 'favicon-frame.png');
  let defaultIcon;
  return async (settings) => {
    if (!demo && settings) {
      try {
        const packageRoot = dirname(resolve(project, settings.manifest));
        const base = await realpath(settings.icon ? project : packageRoot);
        const icon = settings.icon ?? parse(await readFile(join(base, 'Canopus.toml'), 'utf8')).native_app?.icon;
        if (typeof icon === 'string' && icon.trim()) {
          const path = await realpath(resolve(base, icon));
          const local = relative(base, path);
          if (local && !isAbsolute(local) && local !== '..' && !local.startsWith('..' + sep)) {
            const info = await stat(path);
            if (info.isFile() && info.size <= limit) {
              const bytes = await readFile(path);
              const input = extname(path).toLowerCase() === '.bin' ? lvglImage(bytes) : sharp(bytes, { limitInputPixels: limit });
              return await compose(input, frame);
            }
          }
        }
      } catch {
        // Missing manifests, unreadable files and unsupported image formats use the fallback.
      }
    }
    return defaultIcon ??= compose(sharp(join(assets, demo ? 'favicon-demo.png' : 'favicon-fallback.png')), frame);
  };
}
