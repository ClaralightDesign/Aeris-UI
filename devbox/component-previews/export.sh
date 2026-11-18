#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: LVGL_DIR=/path/to/lvgl sh $0 /path/to/component-previews.html" >&2
  exit 2
fi

preview_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
native_src="$preview_dir/../native"
build_dir="${CARGO_TARGET_DIR:-$preview_dir/target}"
mkdir -p "$build_dir"
build_dir=$(CDPATH='' cd -- "$build_dir" && pwd)
export CARGO_TARGET_DIR="$build_dir"

native_build="$build_dir/preview-native"
mkdir -p "$native_build"
for file in Makefile browser.c; do
  if ! cmp -s "$native_src/$file" "$native_build/$file"; then
    cp "$native_src/$file" "$native_build/$file"
  fi
done
if [ ! -e "$native_build/lvgl" ]; then
  ln -s "$(CDPATH='' cd -- "${LVGL_DIR:?Set LVGL_DIR to the LVGL source directory}" && pwd)" "$native_build/lvgl"
fi

if [ -n "${AERIS_PREVIEW_LVGL_LIB:-}" ]; then
  make -C "$native_build" browser.o
  lvgl_lib=$AERIS_PREVIEW_LVGL_LIB
else
  make -C "$native_build"
  lvgl_lib="$native_build/liblvgl.a"
fi

CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS:--C panic=abort}" \
  "${CARGO:-cargo}" build --locked --release --target wasm32-unknown-emscripten --manifest-path "$preview_dir/Cargo.toml"
"${EMCC:-emcc}" -Os -flto -sSINGLE_FILE=1 -sEXPORTED_RUNTIME_METHODS=UTF8ToString,callMain \
  -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=131072 --shell-file "$native_src/shell.html" \
  "$native_build/browser.o" "$build_dir/wasm32-unknown-emscripten/release/libaeris_component_previews.a" \
  "$lvgl_lib" -o "$1"
node --input-type=module - "$preview_dir" "$1" <<'JS'
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [previewDir, output] = process.argv.slice(2);
const { embedDisplays, loadDisplayProfiles } = await import(pathToFileURL(resolve(previewDir, '../native/devices.mjs')));
const devices = loadDisplayProfiles(resolve(previewDir, '../..'));
writeFileSync(output, embedDisplays(readFileSync(output, 'utf8'), devices, true));
JS
