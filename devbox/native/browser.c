/* 浏览器的显示、指针和 LVGL 资源适配。 */

#include "lvgl.h"
#include "src/core/lv_obj_class_private.h"
#include "src/core/lv_obj_scroll_private.h"
#include "src/draw/lv_draw_image_private.h"
#include "src/misc/cache/instance/lv_image_cache.h"
#include <emscripten.h>
#include <stdlib.h>
#include <string.h>

enum {
    BLUR_SCALE = 4, BLUR_RADIUS = 2, BLUR_BRIGHTNESS = 35
};

static uint32_t *framebuffer;
static lv_indev_t *pointer;
static lv_point_t point;
static bool pressed;
static lv_obj_t *wheel_target;
static lv_timer_t *wheel_timer;

typedef struct Bitmap {
    struct Bitmap *next;
    const char *name;
    size_t name_len;
    lv_image_dsc_t image;
} Bitmap;

static Bitmap *bitmaps;
static lv_font_t fonts[7][8];

typedef struct {
    const uint8_t *bitmap;
    int px;
    unsigned weight;
} FontData;

static FontData font_data[7][8];

typedef uint8_t Bgr[3];

static lv_draw_buf_t *blur_bitmap;
static lv_image_dsc_t blur_image;

extern int aeris_devbox_mount(void);

EM_JS(void, canvas_paint, (uint8_t *pixels, int width, int height), {
    Module.galleryPaint(HEAPU8, pixels, width, height);
});

EM_JS(bool, setup_debug_measurements, (void), {
    if (!Module.galleryDebugEnabled) return false;
    Module.galleryWasmCapacity = () => HEAPU8.byteLength;
    return true;
});

EM_JS(void, render_event, (bool started), {
    if (started) Module.galleryRenderStart();
    else Module.galleryRenderReady();
});

EM_JS(uint32_t, browser_ticks, (void), {
    return performance.now() >>> 0;
});

EM_JS(void, browser_time, (char *text), {
    const time = new Date();
    const hours = time.getHours();
    const minutes = time.getMinutes();
    HEAPU8.set([48 + Math.floor(hours / 10), 48 + hours % 10, 58,
                48 + Math.floor(minutes / 10), 48 + minutes % 10, 0], text);
});

EM_JS(bool, browser_prefers_reduced_motion, (void), {
    return matchMedia("(prefers-reduced-motion: reduce)").matches;
});

bool gallery_prefers_reduced_motion(void) { return browser_prefers_reduced_motion(); }
void gallery_time(char *text) { browser_time(text); }
uint32_t gallery_now_ms(void) { return browser_ticks(); }

EM_JS(size_t, browser_parameter, (char *value, size_t capacity), {
    const parameter = new URLSearchParams(location.search).get("scene") || "";
    if (parameter.length > capacity) return capacity + 1;
    const bytes = new TextEncoder().encode(parameter);
    if (bytes.length > capacity) return capacity + 1;
    HEAPU8.set(bytes, value);
    return bytes.length;
});

size_t devbox_scene_id(char *value, size_t capacity) {
    return browser_parameter(value, capacity);
}

EM_JS(size_t, browser_device_id, (char *value, size_t capacity), {
    const id = Module.galleryDevice.id;
    if (id.length > capacity) return capacity + 1;
    HEAPU8.set(new TextEncoder().encode(id), value);
    return id.length;
});

size_t devbox_device_id(char *value, size_t capacity) {
    return browser_device_id(value, capacity);
}

EM_JS(void, browser_display_size, (int *width, int *height), {
    HEAP32[width >>> 2] = Module.galleryDevice.width;
    HEAP32[height >>> 2] = Module.galleryDevice.height;
});

EM_JS(void, show_error, (void), {
    Module.galleryError();
});

static uint16_t read_u16(const uint8_t *data) {
    return data[0] | (data[1] << 8);
}

bool gallery_asset(const char *name, size_t name_len, uint8_t *bytes, size_t len) {
    if (len < 12 || bytes[0] != 0x19 || bytes[1] != 0x10) {
        return false;
    }
    const uint16_t width = read_u16(bytes + 4);
    const uint16_t height = read_u16(bytes + 6);
    const uint16_t stride = read_u16(bytes + 8);
    if (!width || !height || stride != 4 * width || len != 12 + (size_t)stride * height) {
        return false;
    }
    for (Bitmap *asset = bitmaps; asset; asset = asset->next) {
        if (asset->name_len == name_len && memcmp(asset->name, name, name_len) == 0) {
            free(bytes);
            return true;
        }
    }
    Bitmap *asset = malloc(sizeof(*asset) + name_len);
    if (!asset) return false;
    char *name_copy = (char *)(asset + 1);
    memcpy(name_copy, name, name_len);
    asset->next = bitmaps;
    bitmaps = asset;
    asset->name = name_copy;
    asset->name_len = name_len;
    asset->image = (lv_image_dsc_t) {
        .header = {
            .magic = LV_IMAGE_HEADER_MAGIC,
            .cf = LV_COLOR_FORMAT_ARGB8888,
            .w = width,
            .h = height,
            .stride = stride,
        },
        .data_size = len - 12,
        .data = bytes + 12,
    };
    return true;
}

/* 以设备的采样比例和滤波半径生成独立的弹窗背景。 */
static void blur_pass(const Bgr *input, Bgr *output, int width, int height, bool vertical) {
    const int limit = vertical ? height : width;
    for (int y = 0; y < height; y++) {
        for (int x = 0; x < width; x++) {
            const int position = vertical ? y : x;
            const int start = LV_MAX(position - BLUR_RADIUS, 0);
            const int end = LV_MIN(position + BLUR_RADIUS, limit - 1);
            for (int channel = 0; channel < 3; channel++) {
                unsigned sum = 0;
                for (int i = start; i <= end; i++) {
                    sum += input[vertical ? i * width + x : y * width + i][channel];
                }
                output[y * width + x][channel] = sum / (end - start + 1);
            }
        }
    }
}

const lv_image_dsc_t *gallery_capture_backdrop(lv_obj_t *root) {
    lv_draw_buf_t *snapshot = lv_snapshot_take(root, LV_COLOR_FORMAT_XRGB8888);
    if (!snapshot) return NULL;
    const int source_width = snapshot->header.w;
    const int source_height = snapshot->header.h;
    const int width = (source_width + BLUR_SCALE - 1) / BLUR_SCALE;
    const int height = (source_height + BLUR_SCALE - 1) / BLUR_SCALE;
    if (!width || !height) {
        lv_draw_buf_destroy(snapshot);
        return NULL;
    }
    Bgr *pixels = malloc((size_t)width * height * sizeof(Bgr) * 2);
    if (!pixels) {
        lv_draw_buf_destroy(snapshot);
        return NULL;
    }
    Bgr *scratch = pixels + (size_t)width * height;
    for (int y = 0; y < height; y++) {
        for (int x = 0; x < width; x++) {
            // 边缘不足一个采样块时只读取实际像素，兼容不能被 4 整除的屏幕。
            const int block_width = LV_MIN(BLUR_SCALE, source_width - x * BLUR_SCALE);
            const int block_height = LV_MIN(BLUR_SCALE, source_height - y * BLUR_SCALE);
            for (int channel = 0; channel < 3; channel++) {
                unsigned sum = 0;
                for (int dy = 0; dy < block_height; dy++) {
                    const uint8_t *row = snapshot->data + (y * BLUR_SCALE + dy) * snapshot->header.stride;
                    for (int dx = 0; dx < block_width; dx++) {
                        sum += row[(x * BLUR_SCALE + dx) * 4 + channel];
                    }
                }
                pixels[y * width + x][channel] = sum / (block_width * block_height);
            }
        }
    }
    lv_draw_buf_destroy(snapshot);

    blur_pass(pixels, scratch, width, height, false);
    blur_pass(scratch, pixels, width, height, true);
    if (!blur_bitmap || blur_bitmap->header.w != width || blur_bitmap->header.h != height) {
        lv_draw_buf_t *bitmap = lv_draw_buf_create(width, height, LV_COLOR_FORMAT_RGB565, 0);
        if (!bitmap) {
            free(pixels);
            return NULL;
        }
        lv_image_cache_drop(&blur_image);
        if (blur_bitmap) lv_draw_buf_destroy(blur_bitmap);
        blur_bitmap = bitmap;
        blur_image = (lv_image_dsc_t) {
            .header = bitmap->header,
            .data_size = bitmap->data_size,
            .data = bitmap->data,
        };
        // 此处传入的是图片描述符，不能让解码器把它当成完整的 draw buffer。
        blur_image.header.flags &= ~LV_IMAGE_FLAGS_ALLOCATED;
    }
    for (int y = 0; y < height; y++) {
        uint16_t *row = (uint16_t *)(blur_bitmap->data + y * blur_bitmap->header.stride);
        for (int x = 0; x < width; x++) {
            const uint8_t *pixel = pixels[y * width + x];
            const unsigned b = pixel[0] * BLUR_BRIGHTNESS / 100;
            const unsigned g = pixel[1] * BLUR_BRIGHTNESS / 100;
            const unsigned r = pixel[2] * BLUR_BRIGHTNESS / 100;
            row[x] = (r >> 3) << 11 | (g >> 2) << 5 | (b >> 3);
        }
    }
    free(pixels);
    lv_image_cache_drop(&blur_image);
    return &blur_image;
}

/* 使用浏览器的中文字体补齐 Linoval 标签没有的字形。 */
EM_JS(void, browser_glyph, (unsigned letter, int px, unsigned weight, int16_t *metrics, unsigned stride, uint8_t *pixels), {
    const glyphs = Module.galleryGlyphs || (Module.galleryGlyphs = new Map());
    const key = px + ":" + weight + ":" + letter;
    const context = Module.galleryGlyphContext || (Module.galleryGlyphContext =
        document.createElement("canvas").getContext("2d", { willReadFrequently: true }));
    let glyph = glyphs.get(key);
    if (!glyph) {
        const text = String.fromCodePoint(letter);
        const font = weight + ' ' + px + 'px "Aeris MiSans ' + weight +
            '", MiSans, "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif';
        context.font = font;
        const bounds = context.measureText(text);
        const left = Math.floor(-bounds.actualBoundingBoxLeft);
        const right = Math.ceil(bounds.actualBoundingBoxRight);
        const top = Math.floor(-bounds.actualBoundingBoxAscent);
        const bottom = Math.ceil(bounds.actualBoundingBoxDescent);
        glyph = { text, font, metrics: [Math.round(bounds.width), right - left, bottom - top, left, -bottom] };
        if (glyphs.size >= 128) glyphs.delete(glyphs.keys().next().value);
        glyphs.set(key, glyph);
    }
    if (metrics) {
        HEAP16.set(glyph.metrics, metrics >>> 1);
        return;
    }
    const [, width, height, left, offsetY] = glyph.metrics;
    if (!glyph.alpha) {
        context.canvas.width = width;
        context.canvas.height = height;
        context.font = glyph.font;
        context.fillText(glyph.text, -left, height + offsetY);
        const data = context.getImageData(0, 0, width, height).data;
        glyph.alpha = new Uint8Array(width * height);
        for (let i = 0; i < glyph.alpha.length; i++) glyph.alpha[i] = data[i * 4 + 3];
    }
    for (let y = 0; y < height; y++) {
        HEAPU8.set(glyph.alpha.subarray(y * width, (y + 1) * width), pixels + stride * y);
    }
});

static bool is_digits_font(const lv_font_t *font) {
    return font == &fonts[3][1] || font == &fonts[3][5] || font == &fonts[3][6] || font == &fonts[3][7];
}

static bool describe_glyph(const lv_font_t *font, lv_font_glyph_dsc_t *glyph, uint32_t letter, uint32_t next) {
    (void)next;
    const FontData *data = font->dsc;
    size_t index;
    if (letter == 9) index = 0;
    else if (letter >= 32 && letter <= 126) index = letter - 32;
    else if (letter == 0xA0) index = 95;
    else if (letter == 0xB0) index = 96;
    else if (letter == 0xB7) index = 97;
    else if (letter == 0x2013) index = 98;
    else if (letter == 0x2212) index = 99;
    else if (font != &fonts[3][1] && font != &fonts[4][0] &&
             ((letter >= 0x3000 && letter <= 0x303f) ||
              (letter >= 0x3400 && letter <= 0x9fff) ||
              (letter >= 0xff01 && letter <= 0xff60))) {
        int16_t metrics[5];
        browser_glyph(letter, data->px, data->weight, metrics, 0, NULL);
        glyph->adv_w = metrics[0];
        glyph->box_w = metrics[1];
        glyph->box_h = metrics[2];
        glyph->ofs_x = metrics[3];
        glyph->ofs_y = metrics[4];
        glyph->format = LV_FONT_GLYPH_FORMAT_A8;
        glyph->gid.index = 0x10000 + letter;
        return true;
    } else return false;

    if (font == &fonts[6][0]) {
        if (letter == '-') index = 0;
        else if (letter >= '0' && letter <= '9') index = letter - '0' + 1;
        else if (letter == ':') index = 11;
        else return false;
    }
    if (is_digits_font(font)) {
        if (letter == ':') index = 10;
        else if (letter >= '0' && letter <= '9') index = letter - '0';
        else return false;
    }
    if (font == &fonts[4][0] || font == &fonts[4][1] || font == &fonts[4][2]) {
        const char *repertoire = " 'TUeimps";
        const char *found = letter > 0 && letter < 128 ? strchr(repertoire, letter) : NULL;
        if (!found) return false;
        index = (size_t)(found - repertoire);
    }
    const uint8_t *metric = data->bitmap + 2 + 7 * index;
    glyph->adv_w = metric[2] * (letter == 9 ? 2 : 1);
    glyph->box_w = metric[3];
    glyph->box_h = metric[4];
    glyph->ofs_x = (int8_t)metric[5];
    glyph->ofs_y = (int8_t)metric[6];
    glyph->format = LV_FONT_GLYPH_FORMAT_A8;
    glyph->gid.index = read_u16(metric);
    return true;
}

static const void *read_glyph(lv_font_glyph_dsc_t *glyph, lv_draw_buf_t *buffer) {
    if (!buffer || !glyph->box_w || !glyph->box_h) return NULL;
    const lv_font_t *font = glyph->resolved_font;
    const FontData *data = font->dsc;
    if (glyph->gid.index >= 0x10000) {
        browser_glyph(glyph->gid.index - 0x10000, data->px, data->weight, NULL,
                      buffer->header.stride, buffer->data);
        return buffer;
    }
    const size_t count = font == &fonts[6][0] ? 12 : is_digits_font(font) ? 11 :
        (font == &fonts[4][0] || font == &fonts[4][1] || font == &fonts[4][2]) ? 9 : 100;
    const uint8_t *pixels = data->bitmap + 2 + 7 * count + glyph->gid.index;
    for (uint16_t y = 0; y < glyph->box_h; y++) {
        memcpy(buffer->data + y * buffer->header.stride, pixels + y * glyph->box_w, glyph->box_w);
    }
    return buffer;
}

static int font_slot(uint32_t family, int px) {
    if (family < 2) return px == 25 ? 1 : px == 26 ? 2 : px == 22 ? 3 : px == 30 ? 4 : 0;
    if (family == 2) return px == 30 ? 1 : px == 25 ? 2 : px == 34 ? 3 : 0;
    if (family == 3) return px == 40 ? 1 : px == 28 ? 2 : px == 31 ? 3 : px == 38 ? 4 : px == 36 ? 5 : px == 41 ? 6 : px == 49 ? 7 : 0;
    if (family == 4) return px == 39 ? 1 : px == 53 ? 2 : 0;
    return 0;
}

EM_JS(int, record_text_style, (uintptr_t id, int family, int pixels, int atlas_pixels), {
    const styles = Module.galleryTextStyles || (Module.galleryTextStyles = new Map());
    const first = !styles.has(id);
    const rounded = family === 4 || family === 6 || family === 3 && [36, 41, 49].includes(atlas_pixels);
    styles.set(id, { fontSize: pixels, atlasFontSize: atlas_pixels,
        fontFamily: rounded ? "Linoval Sans Rounded" : "Linoval Sans",
        fontWeight: rounded ? 650 : [450, 475, 475, 475, 650, 450, 650][family] });
    return first;
});

EM_JS(void, forget_text_style, (uintptr_t id), { Module.galleryTextStyles?.delete(id); });

static void text_style_deleted(lv_event_t *event) {
    forget_text_style((uintptr_t)lv_event_get_current_target(event));
}

EM_JS(void, record_text_apx, (uintptr_t id, int halves, int pixels), {
    const style = Module.galleryTextStyles?.get(id);
    if (style) { style.fontApx = halves / 2; style.fontSize = pixels; }
});

void gallery_text_apx(lv_obj_t *obj, int halves, int pixels) {
    record_text_apx((uintptr_t)obj, halves, pixels);
}

void gallery_font(uint32_t font, int px, const uint8_t *data) {
    static const unsigned weights[] = { 400, 500, 500, 650, 650, 400, 650 };
    const int slot = font_slot(font, px);
    FontData *style = &font_data[font][slot];
    *style = (FontData) { .bitmap = data, .px = px, .weight = weights[font] };
    fonts[font][slot] = (lv_font_t) {
        .get_glyph_dsc = describe_glyph,
        .get_glyph_bitmap = read_glyph,
        .line_height = data[0],
        .base_line = data[1],
        .underline_position = -1,
        .underline_thickness = 1,
        .dsc = style,
    };
}

static lv_text_align_t text_align(uint32_t align) {
    if (align == 3) return LV_TEXT_ALIGN_RIGHT;
    if (align == 2) return LV_TEXT_ALIGN_CENTER;
    return LV_TEXT_ALIGN_LEFT;
}

int gallery_apply_font(lv_obj_t *obj, uint32_t family, int px) {
    const lv_font_t *font = &fonts[family][font_slot(family, px)];
    const FontData *data = font->dsc;
    if (record_text_style((uintptr_t)obj, family, px, data->px))
        lv_obj_add_event_cb(obj, text_style_deleted, LV_EVENT_DELETE, NULL);
    lv_obj_set_style_text_font(obj, font, 0);
    return font->line_height;
}

lv_obj_t *lvx_object_create(lv_obj_t *parent) {
    lv_obj_t *obj = lv_obj_create(parent);
    lv_obj_remove_style_all(obj);
    return obj;
}

lv_obj_t *lvx_label_create(lv_obj_t *parent) {
    lv_obj_t *label = lv_label_create(parent);
    lv_obj_remove_style_all(label);
    return label;
}

void lvx_label_set_text_align_center(lv_obj_t *obj) {
    lv_obj_set_style_text_align(obj, LV_TEXT_ALIGN_CENTER, 0);
}

void lvx_label_set_long_mode(lv_obj_t *obj, uint32_t mode) {
    lv_label_set_long_mode(obj, mode);
}

EM_JS(void, browser_crop, (int32_t height), {
    Module.galleryCrop(height);
});

void gallery_set_crop_height(int32_t height) {
    browser_crop(height);
}

void gallery_sync_refresh_to_frame(void) {
    /* 有画面变更时，跟随浏览器帧刷新。 */
    lv_timer_set_period(lv_display_get_refr_timer(lv_display_get_default()), 0);
}

lv_obj_t *gallery_screen(void) {
    lv_obj_t *old = lv_screen_active();
    lv_obj_t *root = lvx_object_create(NULL);
    lv_obj_set_style_bg_color(root, lv_color_black(), 0);
    lv_obj_set_style_bg_opa(root, LV_OPA_COVER, 0);
    lv_obj_remove_flag(root, LV_OBJ_FLAG_SCROLLABLE | LV_OBJ_FLAG_CLICKABLE);
    lv_screen_load(root);
    if (old) lv_obj_delete_async(old);
    return root;
}

void lvx_object_set_local_style_u32(lv_obj_t *obj, uint32_t prop, uint32_t value, uint32_t selector) {
    (void)selector;
    switch (prop) {
        case 12: lv_obj_set_style_radius(obj, value, 0); break;
        case 28: lv_obj_set_style_bg_color(obj, lv_color_hex(value), 0); break;
        case 16: lv_obj_set_style_pad_top(obj, value, 0); break;
        case 17: lv_obj_set_style_pad_bottom(obj, value, 0); break;
        case 18: lv_obj_set_style_pad_left(obj, value, 0); break;
        case 19: lv_obj_set_style_pad_right(obj, value, 0); break;
        case 22: lv_obj_set_style_layout(obj, value, 0); break;
        case 29: lv_obj_set_style_bg_opa(obj, value, 0); break;
        case 44: lv_obj_set_style_radius(obj, value, 0); break;
        case 45: lv_obj_set_style_clip_corner(obj, value, 0); break;
        case 48: lv_obj_set_style_border_width(obj, value, 0); break;
        case 49: lv_obj_set_style_border_color(obj, lv_color_hex(value), 0); break;
        case 50: lv_obj_set_style_border_opa(obj, value, 0); break;
        case 52: lv_obj_set_style_border_side(obj, value, 0); break;
        case 68: lv_obj_set_style_image_opa(obj, value, 0); break;
        case 69: lv_obj_set_style_image_recolor(obj, lv_color_hex(value), 0); break;
        case 70: lv_obj_set_style_image_recolor_opa(obj, value, 0); break;
        case 88: lv_obj_set_style_text_color(obj, lv_color_hex(value), 0); break;
        case 89: lv_obj_set_style_text_opa(obj, value, 0); break;
        case 92: lv_obj_set_style_text_line_space(obj, (int32_t)value, 0); break;
        case 94: lv_obj_set_style_text_align(obj, text_align(value), 0); break;
        case 95: lv_obj_set_style_opa(obj, value, 0); break;
        case 106: lv_obj_set_style_translate_x(obj, (int32_t)value, 0); break;
        case 107: lv_obj_set_style_translate_y(obj, (int32_t)value, 0); break;
        case 108: lv_obj_set_style_transform_scale_x(obj, value, 0); break;
        case 109: lv_obj_set_style_transform_scale_y(obj, value, 0); break;
        case 110: lv_obj_set_style_transform_rotation(obj, (int32_t)value, 0); break;
        case 111:
            lv_obj_set_style_transform_pivot_x(obj, value, 0);
            if (lv_obj_check_type(obj, &lv_image_class)) lv_image_set_pivot_x(obj, value);
            break;
        case 112:
            lv_obj_set_style_transform_pivot_y(obj, value, 0);
            if (lv_obj_check_type(obj, &lv_image_class)) lv_image_set_pivot_y(obj, value);
            break;
    }
}

static lv_obj_flag_t flags_from_device(uint32_t flags) {
    lv_obj_flag_t result = 0;
    if (flags & (1 << 1)) result |= LV_OBJ_FLAG_CLICKABLE;
    if (flags & (1 << 4)) result |= LV_OBJ_FLAG_SCROLLABLE;
    if (flags & (1 << 5)) result |= LV_OBJ_FLAG_SCROLL_ELASTIC;
    if (flags & (1 << 6)) result |= LV_OBJ_FLAG_SCROLL_MOMENTUM;
    if (flags & (1 << 7)) result |= LV_OBJ_FLAG_SCROLL_ONE;
    if (flags & (1 << 8)) result |= LV_OBJ_FLAG_SCROLL_CHAIN_HOR;
    if (flags & (1 << 9)) result |= LV_OBJ_FLAG_SCROLL_CHAIN_VER;
    if (flags & (1 << 12)) result |= LV_OBJ_FLAG_SNAPPABLE;
    if (flags & (1 << 13)) result |= LV_OBJ_FLAG_PRESS_LOCK;
    if (flags & (1 << 15)) result |= LV_OBJ_FLAG_GESTURE_BUBBLE;
    if (flags & (1 << 20)) result |= LV_OBJ_FLAG_OVERFLOW_VISIBLE;
    return result;
}

void lvx_object_align(lv_obj_t *obj, uint32_t align, int x, int y) {
    if (obj) lv_obj_align(obj, align == 1 ? LV_ALIGN_TOP_LEFT : LV_ALIGN_CENTER, x, y);
}
void gallery_coords(lv_obj_t *obj, int32_t *points) {
    lv_obj_update_layout(obj);
    lv_area_t bounds;
    lv_obj_get_coords(obj, &bounds);
    points[0] = bounds.x1;
    points[1] = bounds.y1;
    points[2] = bounds.x2;
    points[3] = bounds.y2;
}
void lvx_object_move_to_index(lv_obj_t *obj, int index) { lv_obj_move_to_index(obj, index); }
void lvx_object_set_scroll_snap_x(lv_obj_t *obj, uint32_t snap) { lv_obj_set_scroll_snap_x(obj, snap); }
int lvx_object_scroll_x(lv_obj_t *obj) { return lv_obj_get_scroll_x(obj); }
void lvx_object_scroll_to_x(lv_obj_t *obj, int x, uint32_t animated) {
    lv_obj_scroll_to_x(obj, x, animated ? LV_ANIM_ON : LV_ANIM_OFF);
}
lv_timer_t *lvx_timer_create(lv_timer_cb_t callback, uint32_t ms, void *user) {
    return lv_timer_create(callback, ms, user);
}
void lvx_timer_delete(lv_timer_t *timer) { lv_timer_delete(timer); }
void *devbox_timer_user_data(lv_timer_t *timer) { return lv_timer_get_user_data(timer); }
void devbox_timer_pause(lv_timer_t *timer) { lv_timer_pause(timer); }
void devbox_timer_resume(lv_timer_t *timer) { lv_timer_resume(timer); }
void devbox_timer_reset(lv_timer_t *timer) { lv_timer_reset(timer); }
void lvx_object_scroll_to_y(lv_obj_t *obj, int y, uint32_t animated) {
    lv_obj_scroll_to_y(obj, y, animated ? LV_ANIM_ON : LV_ANIM_OFF);
}

EM_JS(int, storage_read, (const char *key, uint8_t *buffer, size_t capacity), {
    try {
        const value = Module.galleryStorageAccess.getItem(UTF8ToString(key));
        if (value === null) return -2;
        const bytes = atob(value);
        if (!buffer) return bytes.length;
        const size = Math.min(bytes.length, capacity);
        for (let i = 0; i < size; i++) HEAPU8[buffer + i] = bytes.charCodeAt(i);
        return size;
    } catch { return -5; }
});

EM_JS(int, storage_write, (const char *key, const uint8_t *buffer, size_t length), {
    try {
        let bytes = "";
        for (let i = 0; i < length; i++) bytes += String.fromCharCode(HEAPU8[buffer + i]);
        Module.galleryStorageAccess.setItem(UTF8ToString(key), btoa(bytes));
        return 0;
    } catch { return -5; }
});

EM_JS(int, storage_remove, (const char *key), {
    try {
        Module.galleryStorageAccess.removeItem(UTF8ToString(key));
        return 0;
    } catch { return -5; }
});

int devbox_storage_read(const char *key, uint8_t *buffer, size_t capacity) { return storage_read(key, buffer, capacity); }
int devbox_storage_write(const char *key, const uint8_t *buffer, size_t length) { return storage_write(key, buffer, length); }
int devbox_storage_remove(const char *key) { return storage_remove(key); }
void lvx_object_set_size(lv_obj_t *obj, int w, int h) { lv_obj_set_size(obj, w, h); }
void lvx_object_add_flag(lv_obj_t *obj, uint32_t flags) { lv_obj_add_flag(obj, flags_from_device(flags)); }
void lvx_object_clear_flag(lv_obj_t *obj, uint32_t flags) { lv_obj_remove_flag(obj, flags_from_device(flags)); }
void lvx_object_delete(lv_obj_t *obj) { lv_obj_delete(obj); }
void devbox_delete_page(lv_obj_t *obj) {
    lv_obj_add_flag(obj, LV_OBJ_FLAG_HIDDEN);
    lv_obj_delete_async(obj);
}

extern void aeris_devbox_back(void) __attribute__((weak));
EMSCRIPTEN_KEEPALIVE void devbox_back(void) {
    if (aeris_devbox_back) aeris_devbox_back();
}
void lvx_set_hidden(lv_obj_t *obj, uint32_t hidden) {
    if (hidden) lv_obj_add_flag(obj, LV_OBJ_FLAG_HIDDEN);
    else lv_obj_remove_flag(obj, LV_OBJ_FLAG_HIDDEN);
}
lv_obj_t *lvx_image_create(lv_obj_t *parent) { return lv_image_create(parent); }
void lvx_image_set_src(lv_obj_t *obj, const void *source) {
    if (source == &blur_image) {
        lv_image_set_src(obj, source);
        return;
    }
    const char *name = source;
    const size_t len = strlen(name);
    for (Bitmap *asset = bitmaps; asset; asset = asset->next) {
        if (asset->name_len == len && memcmp(asset->name, name, len) == 0) {
            lv_image_set_src(obj, &asset->image);
            return;
        }
    }
}
void lvx_image_set_scale(lv_obj_t *obj, int sx, int sy) {
    lv_image_set_scale_x(obj, sx);
    lv_image_set_scale_y(obj, sy);
}
void lvx_label_set_text(lv_obj_t *obj, const char *text) { lv_label_set_text(obj, text); }
void lvx_object_set_scroll_dir(lv_obj_t *obj, uint32_t direction) {
    lv_obj_set_scroll_dir(obj, direction == 12 ? LV_DIR_VER : LV_DIR_HOR);
}
void lvx_object_set_scrollbar_mode(lv_obj_t *obj, uint32_t mode) {
    lv_obj_set_scrollbar_mode(obj, mode == 0 ? LV_SCROLLBAR_MODE_OFF : LV_SCROLLBAR_MODE_AUTO);
}
int lvx_object_scroll_y(lv_obj_t *obj) { return lv_obj_get_scroll_y(obj); }
void lvx_object_scroll_by(lv_obj_t *obj, int x, int y, uint32_t animated) {
    lv_obj_scroll_by(obj, x, y, animated ? LV_ANIM_ON : LV_ANIM_OFF);
}
void raw_lv_obj_scroll_to(lv_obj_t *obj, int x, int y, uint32_t animated) {
    lv_obj_scroll_to(obj, x, y, animated ? LV_ANIM_ON : LV_ANIM_OFF);
}

static lv_event_code_t device_events[] = {
    [1] = LV_EVENT_PRESSED, [2] = LV_EVENT_PRESSING, [3] = LV_EVENT_PRESS_LOST,
    [4] = LV_EVENT_SHORT_CLICKED, [5] = LV_EVENT_LONG_PRESSED,
    [7] = LV_EVENT_CLICKED, [8] = LV_EVENT_RELEASED,
    [11] = LV_EVENT_SCROLL_END, [12] = LV_EVENT_SCROLL,
    [36] = LV_EVENT_DELETE,
};

void lvx_event_add(lv_obj_t *obj, lv_event_cb_t callback, uint32_t code, void *data) {
    if (code < sizeof(device_events) / sizeof(*device_events) && device_events[code]) {
        lv_obj_add_event_cb(obj, callback, device_events[code], data);
    }
}
uintptr_t lvx_event_get_user_data(lv_event_t *event) { return (uintptr_t)lv_event_get_user_data(event); }
uint32_t lvx_event_get_code(lv_event_t *event) {
    const lv_event_code_t code = lv_event_get_code(event);
    for (size_t i = 1; i < sizeof(device_events) / sizeof(*device_events); i++) {
        if (device_events[i] == code) return i;
    }
    return 0;
}

lv_indev_t *raw_lv_event_get_indev(lv_event_t *event) {
    return lv_event_get_indev(event);
}
void raw_lv_indev_get_point(lv_indev_t *device, lv_point_t *coords) {
    lv_indev_get_point(device, coords);
}

static void read_pointer(lv_indev_t *device, lv_indev_data_t *data) {
    (void)device;
    data->point = point;
    data->state = pressed ? LV_INDEV_STATE_PRESSED : LV_INDEV_STATE_RELEASED;
}

static void wheel_target_deleted(lv_event_t *event) {
    if (lv_event_get_target(event) == wheel_target) wheel_target = NULL;
}

static void finish_wheel(lv_timer_t *timer) {
    lv_timer_pause(timer);
    lv_obj_t *obj = wheel_target;
    if (!obj) return;
    wheel_target = NULL;
    lv_obj_remove_event_cb(obj, wheel_target_deleted);
    if (lv_obj_send_event(obj, LV_EVENT_SCROLL_END, NULL) != LV_RESULT_OK) return;
    if (lv_obj_get_scroll_snap_x(obj) != LV_SCROLL_SNAP_NONE ||
        lv_obj_get_scroll_snap_y(obj) != LV_SCROLL_SNAP_NONE) {
        lv_obj_update_snap(obj, LV_ANIM_ON);
    }
}

EMSCRIPTEN_KEEPALIVE void gallery_pointer(int x, int y, int phase) {
    if (!pointer) return;
    if (phase == 0 && wheel_target) finish_wheel(wheel_timer);
    point = (lv_point_t){ x, y };
    if (phase == 3) {
        lv_indev_wait_release(pointer);
    }
    pressed = phase == 0 || phase == 1;
    lv_indev_read(pointer);
}

static int32_t wheel_room(lv_obj_t *obj, lv_dir_t direction) {
    switch (direction) {
        case LV_DIR_LEFT: return lv_obj_get_scroll_left(obj);
        case LV_DIR_RIGHT: return lv_obj_get_scroll_right(obj);
        case LV_DIR_TOP: return lv_obj_get_scroll_top(obj);
        case LV_DIR_BOTTOM: return lv_obj_get_scroll_bottom(obj);
        default: return 0;
    }
}

static lv_obj_t *pointer_target(int x, int y) {
    if (!pointer) return NULL;
    lv_display_t *display = lv_indev_get_display(pointer);
    lv_obj_t *layers[] = {
        lv_display_get_layer_sys(display), lv_display_get_layer_top(display),
        lv_display_get_screen_active(display), lv_display_get_layer_bottom(display),
    };
    lv_point_t at = { x, y };
    for (size_t i = 0; i < sizeof(layers) / sizeof(layers[0]); i++) {
        lv_obj_update_layout(layers[i]);
        lv_obj_t *hit = lv_indev_search_obj(layers[i], &at);
        if (hit) return hit;
    }
    return NULL;
}

static lv_obj_t *find_wheel_target(lv_obj_t *hit, lv_dir_t dir) {
    const lv_obj_flag_t chain = (dir & LV_DIR_HOR) ? LV_OBJ_FLAG_SCROLL_CHAIN_HOR : LV_OBJ_FLAG_SCROLL_CHAIN_VER;
    for (lv_obj_t *obj = hit; obj; obj = lv_obj_get_parent(obj)) {
        if (lv_obj_has_flag(obj, LV_OBJ_FLAG_SCROLLABLE) && (lv_obj_get_scroll_dir(obj) & dir)) {
            if (wheel_room(obj, dir) > 0) return obj;
        }
        if (!lv_obj_has_flag(obj, chain)) break;
    }
    return NULL;
}

static lv_dir_t wheel_direction(int dx, int dy) {
    return LV_ABS(dx) > LV_ABS(dy) ? (dx < 0 ? LV_DIR_LEFT : LV_DIR_RIGHT)
                                  : (dy < 0 ? LV_DIR_TOP : LV_DIR_BOTTOM);
}

EMSCRIPTEN_KEEPALIVE uintptr_t gallery_reload_hit(int x, int y, int dx, int dy) {
    lv_obj_t *hit = pointer_target(x, y);
    return (uintptr_t)((dx || dy) ? find_wheel_target(hit, wheel_direction(dx, dy)) : hit);
}

EMSCRIPTEN_KEEPALIVE bool gallery_reload_scrolling(void) {
    return wheel_target || (pointer && lv_indev_get_scroll_obj(pointer));
}

EMSCRIPTEN_KEEPALIVE void gallery_reload_scroll(lv_obj_t *obj, int x, int y) {
    if (lv_obj_is_valid(obj) && lv_obj_has_flag(obj, LV_OBJ_FLAG_SCROLLABLE)) {
        lv_obj_stop_scroll_anim(obj);
        // Scroll callbacks may delete the object. The verifier re-reads the tree
        // after each change before operating on another axis or object.
        if (lv_obj_get_scroll_x(obj) != x) lv_obj_scroll_to_x(obj, x, LV_ANIM_OFF);
        else lv_obj_scroll_to_y(obj, y, LV_ANIM_OFF);
    }
}

EMSCRIPTEN_KEEPALIVE void gallery_wheel(int x, int y, int dx, int dy) {
    if (!pointer || pressed || (!dx && !dy)) return;
    const lv_dir_t dir = wheel_direction(dx, dy);
    const bool horizontal = (dir & LV_DIR_HOR) != 0;
    const int delta = horizontal ? dx : dy;
    lv_obj_t *obj = find_wheel_target(pointer_target(x, y), dir);
    if (!obj) return;
    if (obj != wheel_target) {
        if (wheel_target) {
            // Scroll-end callbacks can change the layout or delete objects.
            finish_wheel(wheel_timer);
            gallery_wheel(x, y, dx, dy);
            return;
        }
        wheel_target = obj;
        lv_obj_add_event_cb(obj, wheel_target_deleted, LV_EVENT_DELETE, NULL);
        lv_obj_stop_scroll_anim(obj);
        if (!wheel_target || lv_obj_send_event(obj, LV_EVENT_SCROLL_BEGIN, NULL) != LV_RESULT_OK) return;
    }
    if (!wheel_timer) wheel_timer = lv_timer_create(finish_wheel, 200, NULL);
    lv_timer_reset(wheel_timer);
    lv_timer_resume(wheel_timer);
    const int32_t room = wheel_room(obj, dir);
    const int32_t offset = delta < 0 ? LV_MIN(room, -delta) : -LV_MIN(room, delta);
    lv_obj_scroll_by_raw(obj, horizontal ? offset : 0, horizontal ? 0 : offset);
}

EM_JS(void, browser_panic, (const char *file, size_t length, unsigned line), {
    Module.galleryLog("error", new TextDecoder().decode(HEAPU8.subarray(file, file + length)) + ":" + line);
});

void gallery_panic(const char *file, size_t length, unsigned line) { browser_panic(file, length, line); }

EM_JS(void, add_utility_classes, (uintptr_t id, const char *source, size_t len), {
    if (!Module.galleryUtilityClasses) Module.galleryUtilityClasses = new Map();
    const values = Module.galleryUtilityClasses;
    const text = UTF8ToString(source, len, true).trim();
    const previous = values.get(id);
    if (text) values.set(id, previous ? previous + " " + text : text);
});

EM_JS(void, forget_utility_classes, (uintptr_t id), {
    Module.galleryUtilityClasses?.delete(id);
});

static void utility_classes_deleted(lv_event_t *event) {
    forget_utility_classes((uintptr_t)lv_event_get_current_target(event));
}

void gallery_add_utility_classes(lv_obj_t *obj, const char *source, size_t len, uint32_t first) {
    if (first) lv_obj_add_event_cb(obj, utility_classes_deleted, LV_EVENT_DELETE, NULL);
    add_utility_classes((uintptr_t)obj, source, len);
}

EM_JS(bool, register_layer_metadata, (uintptr_t id, bool circle, uint32_t blur_radius), {
    if (!Module.galleryDebugEnabled) return false;
    const metadata = Module.galleryLayerMetadata || (Module.galleryLayerMetadata = new Map());
    let entry = metadata.get(id);
    const first = !entry;
    if (first) {
        if (!circle && !blur_radius) return false;
        metadata.set(id, entry = {});
    }
    if (circle) entry.circle = true;
    else entry.blurRadius = blur_radius >>> 0;
    return first;
});

EM_JS(void, forget_layer_metadata, (uintptr_t id), { Module.galleryLayerMetadata.delete(id); });

static void layer_metadata_deleted(lv_event_t *event) {
    forget_layer_metadata((uintptr_t)lv_event_get_current_target(event));
}

static void set_layer_metadata(lv_obj_t *obj, bool circle, uint32_t radius) {
    if (obj && register_layer_metadata((uintptr_t)obj, circle, radius)) {
        lv_obj_add_event_cb(obj, layer_metadata_deleted, LV_EVENT_DELETE, NULL);
    }
}

void gallery_inspect_circle(lv_obj_t *obj) {
    set_layer_metadata(obj, true, 0);
}

EMSCRIPTEN_KEEPALIVE void gallery_inspect_backdrop_blur(lv_obj_t *obj, uint32_t radius) {
    set_layer_metadata(obj, false, radius);
}

EM_JS(void, inspect_node,
      (uintptr_t id, uintptr_t parent, int kind, const char *class_name, const char *component, const char *value, size_t length,
       int x, int y, int width, int height,
       int content_x, int content_y, int content_width, int content_height,
       int margin_top, int margin_right, int margin_bottom, int margin_left,
       int hidden, int clickable, int scrollable, int overflow, unsigned events,
       uint32_t background, uint32_t foreground, unsigned background_opacity, unsigned opacity,
       int radius, int border, int border_top, int border_right, int border_bottom, int border_left,
       int font_height, double text_scale, int alignment, unsigned checked, unsigned disabled), {
    const textStyle = kind === 1 ? Module.galleryTextStyles?.get(id) : undefined;
    if (Module.galleryLayerMetadata?.has(id)) events--;
    Module.galleryNodes.push({
        id, parent, kind, className: class_name ? UTF8ToString(class_name) : "",
        component: component ? UTF8ToString(component) : "",
        utilityClasses: Module.galleryUtilityClasses?.get(id),
        value: value ? new TextDecoder().decode(HEAPU8.subarray(value, value + length)) : "",
        x, y, width, height, contentX: content_x, contentY: content_y,
        contentWidth: content_width, contentHeight: content_height,
        margin: [margin_top, margin_right, margin_bottom, margin_left],
        borderWidths: [border_top, border_right, border_bottom, border_left],
        hidden: !!hidden, clickable: !!clickable, scrollable: !!scrollable, overflow: !!overflow, events,
        background: background >>> 0, foreground: foreground >>> 0, backgroundOpacity: background_opacity,
        opacity, radius, border, fontHeight: font_height, alignment,
        ...(textStyle && { fontSize: textStyle.fontSize, fontApx: textStyle.fontApx,
            fontFamily: textStyle.fontFamily, fontWeight: textStyle.fontWeight,
            renderedFontSize: textStyle.atlasFontSize * text_scale }),
        renderedLineHeight: font_height * text_scale,
        checked: !!checked, disabled: !!disabled,
    });
});

EM_JS(uint32_t, inspect_layer_geometry,
      (uintptr_t id, int clip_radius, double a, double b, double c, double d, double e, double f), {
    const node = Module.galleryNodes[Module.galleryNodes.length - 1];
    const metadata = Module.galleryLayerMetadata?.get(id);
    node.layerGeometry = {
        clipRadius: clip_radius,
        ...(metadata?.circle && { outline: "circle" }),
        ...(a !== 1 || b || c || d !== 1 || e || f ? { transform: [a, b, c, d, e, f] } : {}),
    };
    return metadata?.blurRadius || 0;
});

EM_JS(void, inspect_backdrop,
      (int x, int y, int width, int height, int opacity, uint32_t radius), {
    const node = Module.galleryNodes[Module.galleryNodes.length - 1];
    node.layerGeometry.backdrop = {
        x, y, width, height, opacity: opacity / 255, radius: radius >>> 0,
    };
});

static void inspect_geometry(lv_obj_t *obj, const lv_area_t *area, int kind) {
    int32_t width = LV_MAX(1, lv_area_get_width(area));
    int32_t height = LV_MAX(1, lv_area_get_height(area));
    lv_point_t corners[] = {
        {area->x1, area->y1},
        {area->x1 + width, area->y1},
        {area->x1, area->y1 + height},
    };
    lv_obj_transform_point_array(obj, corners, 3, LV_OBJ_POINT_TRANSFORM_FLAG_RECURSIVE);
    double a = (corners[1].x - corners[0].x) / (double)width;
    double b = (corners[1].y - corners[0].y) / (double)width;
    double c = (corners[2].x - corners[0].x) / (double)height;
    double d = (corners[2].y - corners[0].y) / (double)height;
    int radius = lv_obj_get_style_clip_corner(obj, LV_PART_MAIN) ? lv_obj_get_style_radius(obj, LV_PART_MAIN) : 0;
    uint32_t blur_radius = inspect_layer_geometry((uintptr_t)obj, radius, a, b, c, d,
                                corners[0].x - a * area->x1 - c * area->y1,
                                corners[0].y - b * area->x1 - d * area->y1);
    if (kind != 2 || !blur_bitmap || lv_image_get_src(obj) != &blur_image) {
        if (blur_radius) {
            inspect_backdrop(area->x1, area->y1, lv_area_get_width(area), lv_area_get_height(area), LV_OPA_COVER, blur_radius);
        }
        return;
    }
    lv_point_t pivot;
    lv_image_get_pivot(obj, &pivot);
    int32_t source_width = blur_image.header.w;
    int32_t source_height = blur_image.header.h;
    lv_area_t source = {0, 0, source_width - 1, source_height - 1};
    lv_area_align(area, &source, (lv_align_t)lv_image_get_inner_align(obj),
                  lv_image_get_offset_x(obj), lv_image_get_offset_y(obj));
    lv_area_t drawn;
    lv_image_buf_get_transformed_area(&drawn, source_width, source_height, lv_image_get_rotation(obj),
                                      lv_image_get_scale_x(obj), lv_image_get_scale_y(obj), &pivot);
    lv_area_move(&drawn, source.x1, source.y1);
    inspect_backdrop(drawn.x1, drawn.y1, lv_area_get_width(&drawn), lv_area_get_height(&drawn),
                     lv_obj_get_style_image_opa(obj, LV_PART_MAIN), blur_radius ? blur_radius : BLUR_RADIUS * BLUR_SCALE);
}

static bool is_nine_patch_slice(lv_obj_t *obj) {
    const char *name = lv_obj_get_name(obj);
    return name && strcmp(name, "AerisUI.NinePatchSlice") == 0;
}

EM_JS(bool, reload_metadata, (uintptr_t id, const char *label, size_t length, int value, bool set_value), {
    if (!Module.galleryReloadEnabled) return false;
    if (!Module.galleryReloadMetadata) Module.galleryReloadMetadata = new Map();
    let metadata = Module.galleryReloadMetadata.get(id);
    const first = !metadata;
    if (first) Module.galleryReloadMetadata.set(id, metadata = {});
    if (set_value) metadata.state = value;
    else metadata.label = new TextDecoder().decode(HEAPU8.subarray(label, label + length));
    return first;
});

EM_JS(void, clear_reload_metadata, (uintptr_t id), {
    Module.galleryReloadMetadata.delete(id);
});

static void reload_metadata_deleted(lv_event_t *event) {
    clear_reload_metadata((uintptr_t)lv_event_get_target(event));
}

void gallery_reload_label(lv_obj_t *obj, const char *label, size_t length) {
    if (reload_metadata((uintptr_t)obj, label, length, 0, false)) {
        lv_obj_add_event_cb(obj, reload_metadata_deleted, LV_EVENT_DELETE, NULL);
    }
}

void gallery_reload_value(lv_obj_t *obj, int value) {
    if (reload_metadata((uintptr_t)obj, NULL, 0, value, true)) {
        lv_obj_add_event_cb(obj, reload_metadata_deleted, LV_EVENT_DELETE, NULL);
    }
}

EM_JS(void, inspect_reload_state, (uintptr_t id, int x, int y, bool scroll, bool snap, bool moving), {
    const node = Module.galleryNodes[Module.galleryNodes.length - 1];
    const metadata = Module.galleryReloadMetadata?.get(id);
    if (metadata) Object.assign(node, metadata);
    if (scroll) node.scroll = [x, y];
    if (snap) node.snap = true;
    if (moving) node.moving = true;
});

static void inspect_tree(lv_obj_t *obj, lv_obj_t *parent, bool hidden, const char *root_class, bool reload, bool geometry) {
    lv_area_t area, content;
    lv_obj_get_coords(obj, &area);
    lv_obj_get_content_coords(obj, &content);
    hidden = hidden || lv_obj_has_flag(obj, LV_OBJ_FLAG_HIDDEN);
    int kind = !parent ? 0 : lv_obj_has_class(obj, &lv_label_class) ? 1 :
        lv_obj_has_class(obj, &lv_image_class) ? 2 : 3;
    const char *value = kind == 1 ? lv_label_get_text(obj) : NULL;
    const lv_font_t *font = lv_obj_get_style_text_font(obj, LV_PART_MAIN);
    const char *class_name = parent ? lv_obj_get_class(obj)->name : root_class;
    const uint32_t count = lv_obj_get_child_count(obj);
    for (uint32_t index = 0; index < count; index++) {
        if (is_nine_patch_slice(lv_obj_get_child(obj, index))) {
            class_name = "NinePatch";
            break;
        }
    }
    const char *name = lv_obj_get_name(obj);
    const char *component = name && strncmp(name, "AerisUI.", 8) == 0 ? name : NULL;
    const lv_border_side_t border_sides = lv_obj_get_style_border_side(obj, LV_PART_MAIN);
    const int border = lv_obj_get_style_border_width(obj, LV_PART_MAIN);
    double text_scale = 1.0;
    for (lv_obj_t *ancestor = obj; ancestor; ancestor = lv_obj_get_parent(ancestor)) {
        text_scale *= lv_obj_get_style_transform_scale_y(ancestor, LV_PART_MAIN) / 256.0;
    }
    inspect_node((uintptr_t)obj, (uintptr_t)parent, kind, class_name, component, value, value ? strlen(value) : 0,
                 area.x1, area.y1, lv_area_get_width(&area), lv_area_get_height(&area),
                 content.x1, content.y1, lv_area_get_width(&content), lv_area_get_height(&content),
                 lv_obj_get_style_margin_top(obj, LV_PART_MAIN), lv_obj_get_style_margin_right(obj, LV_PART_MAIN),
                 lv_obj_get_style_margin_bottom(obj, LV_PART_MAIN), lv_obj_get_style_margin_left(obj, LV_PART_MAIN),
                 hidden, lv_obj_has_flag(obj, LV_OBJ_FLAG_CLICKABLE), lv_obj_has_flag(obj, LV_OBJ_FLAG_SCROLLABLE),
                 lv_obj_has_flag(obj, LV_OBJ_FLAG_OVERFLOW_VISIBLE), lv_obj_get_event_count(obj),
                 lv_color_to_u32(lv_obj_get_style_bg_color(obj, LV_PART_MAIN)) & 0xffffff,
                 lv_color_to_u32(lv_obj_get_style_text_color(obj, LV_PART_MAIN)) & 0xffffff,
                 lv_obj_get_style_bg_opa(obj, LV_PART_MAIN), lv_obj_get_style_opa_recursive(obj, LV_PART_MAIN),
                 lv_obj_get_style_radius(obj, LV_PART_MAIN), border,
                 border_sides & LV_BORDER_SIDE_TOP ? border : 0, border_sides & LV_BORDER_SIDE_RIGHT ? border : 0,
                 border_sides & LV_BORDER_SIDE_BOTTOM ? border : 0, border_sides & LV_BORDER_SIDE_LEFT ? border : 0,
                 font ? font->line_height : 0, text_scale, lv_obj_get_style_text_align(obj, LV_PART_MAIN),
                 lv_obj_has_state(obj, LV_STATE_CHECKED), lv_obj_has_state(obj, LV_STATE_DISABLED));
    if (geometry && !hidden) inspect_geometry(obj, &area, kind);
    if (reload) {
        bool scrollable = kind != 1 && lv_obj_has_flag(obj, LV_OBJ_FLAG_SCROLLABLE);
        int x = scrollable ? lv_obj_get_scroll_x(obj) : 0;
        int y = scrollable ? lv_obj_get_scroll_y(obj) : 0;
        lv_dir_t dir = scrollable ? lv_obj_get_scroll_dir(obj) : LV_DIR_NONE;
        bool scroll = x || y
            || ((dir & LV_DIR_LEFT) && lv_obj_get_scroll_left(obj) > 0)
            || ((dir & LV_DIR_RIGHT) && lv_obj_get_scroll_right(obj) > 0)
            || ((dir & LV_DIR_TOP) && lv_obj_get_scroll_top(obj) > 0)
            || ((dir & LV_DIR_BOTTOM) && lv_obj_get_scroll_bottom(obj) > 0);
        bool snap = scroll && (lv_obj_get_scroll_snap_x(obj) != LV_SCROLL_SNAP_NONE
            || lv_obj_get_scroll_snap_y(obj) != LV_SCROLL_SNAP_NONE);
        lv_point_t end = {x, y};
        if (scroll && !hidden) lv_obj_get_scroll_end(obj, &end);
        inspect_reload_state((uintptr_t)obj, x, y, scroll, snap, end.x != x || end.y != y);
    }
    for (uint32_t index = 0; index < count; index++) {
        lv_obj_t *child = lv_obj_get_child(obj, index);
        if (!is_nine_patch_slice(child)) inspect_tree(child, obj, hidden, NULL, reload, geometry);
    }
}

EMSCRIPTEN_KEEPALIVE void gallery_inspect(bool geometry) {
    lv_obj_t *root = lv_screen_active();
    if (root) {
        lv_obj_update_layout(root);
        inspect_tree(root, NULL, false, "lv_screen", false, geometry);
    }
}

EMSCRIPTEN_KEEPALIVE void gallery_inspect_overlay(bool bottom) {
    lv_display_t *display = lv_display_get_default();
    if (!display) return;
    lv_obj_t *roots[] = {
        lv_display_get_layer_bottom(display), lv_display_get_layer_top(display), lv_display_get_layer_sys(display),
    };
    const char *classes[] = { "lv_layer_bottom", "lv_layer_top", "lv_layer_sys" };
    for (size_t i = bottom ? 0 : 1; i < (bottom ? 1 : 3); i++) {
        lv_obj_t *root = roots[i];
        if (!lv_obj_get_child_count(root) && !lv_obj_get_style_bg_opa(root, LV_PART_MAIN)) continue;
        lv_obj_update_layout(root);
        inspect_tree(root, NULL, false, classes[i], false, true);
    }
}

EMSCRIPTEN_KEEPALIVE void gallery_reload_inspect(void) {
    lv_display_t *display = lv_display_get_default();
    if (!display) return;
    lv_obj_t *roots[] = {
        lv_display_get_screen_active(display), lv_display_get_layer_bottom(display),
        lv_display_get_layer_top(display), lv_display_get_layer_sys(display),
    };
    const char *classes[] = { "lv_screen", "lv_layer_bottom", "lv_layer_top", "lv_layer_sys" };
    for (size_t i = 0; i < sizeof(roots) / sizeof(roots[0]); i++) {
        lv_obj_update_layout(roots[i]);
        inspect_tree(roots[i], NULL, false, classes[i], true, false);
    }
}

static void display_render_event(lv_event_t *event) {
    render_event(lv_event_get_code(event) == LV_EVENT_RENDER_START);
}

static void flush_display(lv_display_t *display, const lv_area_t *area, uint8_t *pixels) {
    (void)area;
    canvas_paint(pixels, lv_display_get_horizontal_resolution(display),
                 lv_display_get_vertical_resolution(display));
    lv_display_flush_ready(display);
}

static void draw(void) { lv_timer_handler(); }

int main(void) {
    int width, height;
    browser_display_size(&width, &height);
    if (width <= 0 || width > 1024 || height <= 0 || height > 1024) {
        show_error();
        return 1;
    }
    const size_t buffer_size = sizeof(*framebuffer) * width * height;
    framebuffer = malloc(buffer_size);
    if (!framebuffer) {
        show_error();
        return 1;
    }
    lv_init();
    lv_tick_set_cb(browser_ticks);
    lv_display_t *display = lv_display_create(width, height);
    if (!display) {
        free(framebuffer);
        show_error();
        return 1;
    }
    lv_display_set_color_format(display, LV_COLOR_FORMAT_ARGB8888);
    lv_display_set_buffers(display, framebuffer, NULL, buffer_size, LV_DISPLAY_RENDER_MODE_FULL);
    lv_display_set_flush_cb(display, flush_display);
    if (setup_debug_measurements()) {
        lv_display_add_event_cb(display, display_render_event, LV_EVENT_RENDER_START, NULL);
        lv_display_add_event_cb(display, display_render_event, LV_EVENT_RENDER_READY, NULL);
    }

    pointer = lv_indev_create();
    lv_indev_set_type(pointer, LV_INDEV_TYPE_POINTER);
    lv_indev_set_read_cb(pointer, read_pointer);
    lv_indev_set_mode(pointer, LV_INDEV_MODE_EVENT);

    if (aeris_devbox_mount() != 0) {
        show_error();
        return 1;
    }
    emscripten_set_main_loop(draw, 0, 1);
    return 0;
}
