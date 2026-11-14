export type PixelSize = { width: number; height: number };
export type RecordedFrame = { blob: Blob; at: number };

export function downloadBlob(blob: Blob, filename: string) {
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(link.href), 10000);
}

async function framePixels(blob: Blob, size: PixelSize) {
  const bitmap = await createImageBitmap(blob);
  const canvas = document.createElement('canvas');
  canvas.width = size.width;
  canvas.height = size.height;
  const context = canvas.getContext('2d', { willReadFrequently: true })!;
  context.drawImage(bitmap, 0, 0, size.width, size.height);
  bitmap.close();
  return context.getImageData(0, 0, size.width, size.height);
}

/** Median-cut palette from sampled pixels of the first captured frame. */
function buildPalette(pixels: Uint8ClampedArray, count = 256) {
  const samples: number[] = [];
  const stride = Math.max(1, Math.floor(pixels.length / 4 / 16000));
  for (let offset = 0; offset < pixels.length; offset += stride * 4) {
    samples.push((pixels[offset] << 16) | (pixels[offset + 1] << 8) | pixels[offset + 2]);
  }
  if (!samples.length) samples.push(0);
  let boxes: number[][] = [samples];
  while (boxes.length < count) {
    let target = -1;
    let spread = -1;
    let channel = 0;
    for (let index = 0; index < boxes.length; index++) {
      const box = boxes[index];
      if (box.length < 2) continue;
      const min = [255, 255, 255];
      const max = [0, 0, 0];
      for (const color of box) {
        const rgb = [color >> 16, (color >> 8) & 255, color & 255];
        for (let c = 0; c < 3; c++) {
          if (rgb[c] < min[c]) min[c] = rgb[c];
          if (rgb[c] > max[c]) max[c] = rgb[c];
        }
      }
      for (let c = 0; c < 3; c++) {
        const range = max[c] - min[c];
        if (range > spread) {
          spread = range;
          target = index;
          channel = c;
        }
      }
    }
    if (target < 0 || spread <= 0) break;
    const box = boxes[target];
    box.sort((a, b) => ((a >> (16 - channel * 8)) & 255) - ((b >> (16 - channel * 8)) & 255));
    const middle = box.length >> 1;
    boxes.splice(target, 1, box.slice(0, middle), box.slice(middle));
  }
  const table: number[] = [];
  for (const box of boxes) {
    let red = 0;
    let green = 0;
    let blue = 0;
    for (const color of box) {
      red += color >> 16;
      green += (color >> 8) & 255;
      blue += color & 255;
    }
    const length = box.length || 1;
    table.push(Math.round(red / length), Math.round(green / length), Math.round(blue / length));
  }
  while (table.length < count * 3) table.push(0, 0, 0);
  return table;
}

const GIF_CODE_SIZE = 8;
const GIF_CLEAR = 1 << GIF_CODE_SIZE;
const GIF_END = GIF_CLEAR + 1;

function lzw(indices: Uint8Array) {
  const output: number[] = [];
  let buffer = 0;
  let bits = 0;
  let codeSize = GIF_CODE_SIZE + 1;
  let nextCode = GIF_END + 1;
  const table = new Map<number, number>();
  const write = (code: number) => {
    buffer |= code << bits;
    bits += codeSize;
    while (bits >= 8) {
      output.push(buffer & 0xff);
      buffer >>>= 8;
      bits -= 8;
    }
  };
  write(GIF_CLEAR);
  let prefix = indices[0];
  for (let index = 1; index < indices.length; index++) {
    const value = indices[index];
    const key = (prefix << 8) | value;
    const found = table.get(key);
    if (found !== undefined) {
      prefix = found;
      continue;
    }
    write(prefix);
    if (nextCode < 4096) {
      if (nextCode >= 1 << codeSize && codeSize < 12) codeSize++;
      table.set(key, nextCode++);
    } else {
      write(GIF_CLEAR);
      table.clear();
      nextCode = GIF_END + 1;
      codeSize = GIF_CODE_SIZE + 1;
    }
    prefix = value;
  }
  write(prefix);
  write(GIF_END);
  if (bits > 0) output.push(buffer & 0xff);
  return Uint8Array.from(output);
}

function pushColorTable(target: number[], palette: number[]) {
  for (let index = 0; index < 256; index++) {
    target.push(palette[index * 3] ?? 0, palette[index * 3 + 1] ?? 0, palette[index * 3 + 2] ?? 0);
  }
}

function pushSubBlocks(target: number[], data: Uint8Array) {
  for (let offset = 0; offset < data.length; offset += 255) {
    const chunk = data.subarray(offset, offset + 255);
    target.push(chunk.length);
    for (const byte of chunk) target.push(byte);
  }
  target.push(0);
}

export async function encodeGif(frames: RecordedFrame[], size: PixelSize, onProgress?: (ratio: number) => void) {
  if (!frames.length) throw new Error('没有可导出的帧。');
  const first = await framePixels(frames[0].blob, size);
  const palette = buildPalette(first.data);
  const lookup = new Uint8Array(32768);
  for (let red = 0; red < 32; red++) {
    for (let green = 0; green < 32; green++) {
      for (let blue = 0; blue < 32; blue++) {
        const r = red * 8 + 4;
        const g = green * 8 + 4;
        const b = blue * 8 + 4;
        let best = 0;
        let distance = Infinity;
        for (let index = 0; index < 256; index++) {
          const dr = r - palette[index * 3];
          const dg = g - palette[index * 3 + 1];
          const db = b - palette[index * 3 + 2];
          const current = dr * dr + dg * dg + db * db;
          if (current < distance) {
            distance = current;
            best = index;
          }
        }
        lookup[(red << 10) | (green << 5) | blue] = best;
      }
    }
  }
  const bytes: number[] = [];
  const text = (value: string) => {
    for (const character of value) bytes.push(character.charCodeAt(0));
  };
  const short = (value: number) => bytes.push(value & 0xff, (value >> 8) & 0xff);
  text('GIF89a');
  short(size.width);
  short(size.height);
  bytes.push(0xf7, 0, 0);
  pushColorTable(bytes, palette);
  bytes.push(0x21, 0xff, 0x0b);
  text('NETSCAPE2.0');
  bytes.push(3, 1, 0, 0, 0);
  for (let frameIndex = 0; frameIndex < frames.length; frameIndex++) {
    const frame = frames[frameIndex];
    const next = frames[frameIndex + 1]?.at ?? frame.at + 100;
    const delay = Math.max(2, Math.min(6000, Math.round((next - frame.at) / 10)));
    bytes.push(0x21, 0xf9, 4, 4, delay & 0xff, delay >> 8, 0, 0);
    bytes.push(0x2c);
    short(0);
    short(0);
    short(size.width);
    short(size.height);
    bytes.push(0);
    bytes.push(GIF_CODE_SIZE);
    const pixels = frameIndex === 0 ? first : await framePixels(frame.blob, size);
    const indices = new Uint8Array(size.width * size.height);
    for (let index = 0, offset = 0; index < indices.length; index++, offset += 4) {
      const red = pixels.data[offset] >> 3;
      const green = pixels.data[offset + 1] >> 3;
      const blue = pixels.data[offset + 2] >> 3;
      indices[index] = lookup[(red << 10) | (green << 5) | blue];
    }
    pushSubBlocks(bytes, lzw(indices));
    onProgress?.((frameIndex + 1) / frames.length);
  }
  bytes.push(0x3b);
  return new Blob([Uint8Array.from(bytes)], { type: 'image/gif' });
}

function readFourCC(view: DataView, offset: number) {
  return String.fromCharCode(
    view.getUint8(offset),
    view.getUint8(offset + 1),
    view.getUint8(offset + 2),
    view.getUint8(offset + 3),
  );
}

function writeFourCC(view: DataView, offset: number, value: string) {
  for (let index = 0; index < 4; index++) view.setUint8(offset + index, value.charCodeAt(index));
}

function uint24(view: DataView, offset: number, value: number) {
  view.setUint8(offset, value & 0xff);
  view.setUint8(offset + 1, (value >> 8) & 0xff);
  view.setUint8(offset + 2, (value >> 16) & 0xff);
}

function webpFrameChunk(buffer: ArrayBuffer) {
  const view = new DataView(buffer);
  if (readFourCC(view, 0) !== 'RIFF' || readFourCC(view, 8) !== 'WEBP') {
    throw new Error('无法读取 WebP 帧。');
  }
  let offset = 12;
  while (offset + 8 <= buffer.byteLength) {
    const id = readFourCC(view, offset);
    const length = view.getUint32(offset + 4, true);
    if (id === 'VP8 ' || id === 'VP8L') {
      const padded = length + (length & 1);
      const chunk = new Uint8Array(8 + padded);
      chunk.set(new Uint8Array(buffer, offset, 8 + length));
      return chunk;
    }
    offset += 8 + length + (length & 1);
  }
  throw new Error('WebP 帧缺少图像数据。');
}

function makeChunk(id: string, payload: Uint8Array) {
  const padded = payload.length + (payload.length & 1);
  const chunk = new Uint8Array(8 + padded);
  const view = new DataView(chunk.buffer);
  writeFourCC(view, 0, id);
  view.setUint32(4, payload.length, true);
  chunk.set(payload, 8);
  return chunk;
}

export async function encodeAnimatedWebp(frames: RecordedFrame[], size: PixelSize, onProgress?: (ratio: number) => void) {
  if (!frames.length) throw new Error('没有可导出的帧。');
  const frameChunks = await Promise.all(frames.map((frame) => frame.blob.arrayBuffer().then(webpFrameChunk)));
  const vp8x = makeChunk('VP8X', new Uint8Array(10));
  const vp8xView = new DataView(vp8x.buffer, 8);
  vp8xView.setUint8(0, 0x02);
  uint24(vp8xView, 4, size.width - 1);
  uint24(vp8xView, 7, size.height - 1);
  const anim = makeChunk('ANIM', new Uint8Array(6));
  new DataView(anim.buffer, 8).setUint16(4, 0, true);
  const animated = frames.map((frame, index) => {
    const next = frames[index + 1]?.at ?? frame.at + 100;
    const duration = Math.max(10, Math.min(65535, Math.round(next - frame.at)));
    const frameChunk = frameChunks[index];
    const payload = new Uint8Array(16 + frameChunk.length);
    const view = new DataView(payload.buffer);
    uint24(view, 0, 0);
    uint24(view, 3, 0);
    uint24(view, 6, size.width - 1);
    uint24(view, 9, size.height - 1);
    uint24(view, 12, duration);
    view.setUint8(15, 0x02);
    payload.set(frameChunk, 16);
    return makeChunk('ANMF', payload);
  });
  const chunks = [vp8x, anim, ...animated];
  const total = 4 + chunks.reduce((length, chunk) => length + chunk.length, 0);
  const output = new Uint8Array(8 + total);
  const view = new DataView(output.buffer);
  writeFourCC(view, 0, 'RIFF');
  view.setUint32(4, total, true);
  writeFourCC(view, 8, 'WEBP');
  let offset = 12;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.length;
  }
  onProgress?.(1);
  return new Blob([output], { type: 'image/webp' });
}
