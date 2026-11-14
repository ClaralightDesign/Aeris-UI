/* 浏览器的显示、指针和 LVGL 资源适配。 */

#include "lvgl.h"
#include "src/misc/cache/instance/lv_image_cache.h"
#include <emscripten.h>
#include <stdlib.h>
#include <string.h>

enum {
    WIDTH = 336, HEIGHT = 480,
    BLUR_SCALE = 4, BLUR_RADIUS = 2, BLUR_BRIGHTNESS = 35,
    BLUR_WIDTH = WIDTH / BLUR_SCALE, BLUR_HEIGHT = HEIGHT / BLUR_SCALE
};

static uint32_t framebuffer[WIDTH * HEIGHT];
static lv_indev_t *pointer;
static lv_point_t point;
static bool pressed;

typedef struct Bitmap {
    struct Bitmap *next;
    const char *name;
    size_t name_len;
    lv_image_dsc_t image;
} Bitmap;

static Bitmap *bitmaps;
static lv_font_t fonts[5][3];

typedef uint8_t Bgr[3];

static Bgr blur_pixels[BLUR_HEIGHT][BLUR_WIDTH];
static Bgr blur_scratch[BLUR_HEIGHT][BLUR_WIDTH];
static uint16_t blur_bitmap[BLUR_HEIGHT][BLUR_WIDTH];
static const lv_image_dsc_t blur_image = {
    .header = {
        .magic = LV_IMAGE_HEADER_MAGIC,
        .cf = LV_COLOR_FORMAT_RGB565,
        .w = BLUR_WIDTH,
        .h = BLUR_HEIGHT,
        .stride = sizeof(blur_bitmap[0]),
    },
    .data_size = sizeof(blur_bitmap),
    .data = (const uint8_t *)blur_bitmap,
};

extern int aeris_devbox_mount(void);

EM_JS(void, canvas_paint, (uint8_t *pixels, int width, int height), {
    Module.galleryPaint(HEAPU8, pixels, width, height);
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
static void blur_pass(Bgr input[BLUR_HEIGHT][BLUR_WIDTH],
                      Bgr output[BLUR_HEIGHT][BLUR_WIDTH], bool vertical) {
    const int limit = vertical ? BLUR_HEIGHT : BLUR_WIDTH;
    for (int y = 0; y < BLUR_HEIGHT; y++) {
        for (int x = 0; x < BLUR_WIDTH; x++) {
            const int position = vertical ? y : x;
            const int start = LV_MAX(position - BLUR_RADIUS, 0);
            const int end = LV_MIN(position + BLUR_RADIUS, limit - 1);
            for (int channel = 0; channel < 3; channel++) {
                unsigned sum = 0;
                for (int i = start; i <= end; i++) {
                    sum += vertical ? input[i][x][channel] : input[y][i][channel];
                }
                output[y][x][channel] = sum / (end - start + 1);
            }
        }
    }
}

const lv_image_dsc_t *gallery_capture_backdrop(lv_obj_t *root) {
    lv_draw_buf_t *snapshot = lv_snapshot_take(root, LV_COLOR_FORMAT_XRGB8888);
    if (!snapshot) return NULL;
    if (snapshot->header.w != WIDTH || snapshot->header.h != HEIGHT) {
        lv_draw_buf_destroy(snapshot);
        return NULL;
    }

    for (int y = 0; y < BLUR_HEIGHT; y++) {
        for (int x = 0; x < BLUR_WIDTH; x++) {
            for (int channel = 0; channel < 3; channel++) {
                unsigned sum = 0;
                for (int dy = 0; dy < BLUR_SCALE; dy++) {
                    const uint8_t *row = snapshot->data + (y * BLUR_SCALE + dy) * snapshot->header.stride;
                    for (int dx = 0; dx < BLUR_SCALE; dx++) {
                        sum += row[(x * BLUR_SCALE + dx) * 4 + channel];
                    }
                }
                blur_pixels[y][x][channel] = sum / (BLUR_SCALE * BLUR_SCALE);
            }
        }
    }
    lv_draw_buf_destroy(snapshot);

    blur_pass(blur_pixels, blur_scratch, false);
    blur_pass(blur_scratch, blur_pixels, true);
    for (int y = 0; y < BLUR_HEIGHT; y++) {
        for (int x = 0; x < BLUR_WIDTH; x++) {
            const uint8_t *pixel = blur_pixels[y][x];
            const unsigned b = pixel[0] * BLUR_BRIGHTNESS / 100;
            const unsigned g = pixel[1] * BLUR_BRIGHTNESS / 100;
            const unsigned r = pixel[2] * BLUR_BRIGHTNESS / 100;
            blur_bitmap[y][x] = (r >> 3) << 11 | (g >> 2) << 5 | (b >> 3);
        }
    }
    lv_image_cache_drop(&blur_image);
    return &blur_image;
}

/* 使用浏览器的中文字体补齐 Linoval 标签没有的字形。 */
EM_JS(void, browser_glyph, (unsigned letter, unsigned height, unsigned baseline, unsigned stride, uint8_t *pixels), {
    const glyphs = Module.galleryGlyphs || (Module.galleryGlyphs = new Map());
    const key = height + ":" + baseline + ":" + letter;
    let alpha = glyphs.get(key);
    if (!alpha) {
        const canvas = Module.galleryGlyphCanvas || (Module.galleryGlyphCanvas = document.createElement("canvas"));
        canvas.width = height;
        canvas.height = height;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        context.font = '500 ' + Math.round(height * 0.9) + 'px MiSans, "PingFang SC", "Noto Sans CJK SC", sans-serif';
        context.textAlign = "center";
        context.fillText(String.fromCodePoint(letter), height / 2, height - baseline);
        const data = context.getImageData(0, 0, height, height).data;
        alpha = new Uint8Array(height * height);
        for (let i = 0; i < alpha.length; i++) alpha[i] = data[i * 4 + 3];
        if (glyphs.size >= 128) glyphs.delete(glyphs.keys().next().value);
        glyphs.set(key, alpha);
    }
    for (let y = 0; y < height; y++) {
        HEAPU8.set(alpha.subarray(y * height, (y + 1) * height), pixels + stride * y);
    }
});

static bool describe_glyph(const lv_font_t *font, lv_font_glyph_dsc_t *glyph, uint32_t letter, uint32_t next) {
    (void)next;
    size_t index;
    if (letter == 9) index = 0;
    else if (letter >= 32 && letter <= 126) index = letter - 32;
    else if (letter == 0xA0) index = 95;
    else if (letter == 0xB0) index = 96;
    else if (letter == 0xB7) index = 97;
    else if (letter == 0x2013) index = 98;
    else if (letter == 0x2212) index = 99;
    else if (letter >= 0x3400 && letter <= 0x9fff && font != &fonts[3][1] && font != &fonts[4][0]) {
        glyph->adv_w = font->line_height;
        glyph->box_w = font->line_height;
        glyph->box_h = font->line_height;
        glyph->ofs_x = 0;
        glyph->ofs_y = -font->base_line;
        glyph->format = LV_FONT_GLYPH_FORMAT_A8;
        glyph->gid.index = 0x10000 + letter;
        return true;
    } else return false;

    const uint8_t *data = font->dsc;
    if (font == &fonts[3][1]) {
        if (letter == ':') index = 10;
        else if (letter >= '0' && letter <= '9') index = letter - '0';
        else return false;
    }
    if (font == &fonts[4][0]) {
        const char *repertoire = " 'TUeimps";
        const char *found = letter > 0 && letter < 128 ? strchr(repertoire, letter) : NULL;
        if (!found) return false;
        index = (size_t)(found - repertoire);
    }
    const uint8_t *metric = data + 2 + 7 * index;
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
    if (glyph->gid.index >= 0x10000) {
        browser_glyph(glyph->gid.index - 0x10000, font->line_height, font->base_line,
                      buffer->header.stride, buffer->data);
        return buffer;
    }
    const uint8_t *data = font->dsc;
    const size_t count = font == &fonts[3][1] ? 11 : font == &fonts[4][0] ? 9 : 100;
    const uint8_t *pixels = data + 2 + 7 * count + glyph->gid.index;
    for (uint16_t y = 0; y < glyph->box_h; y++) {
        memcpy(buffer->data + y * buffer->header.stride, pixels + y * glyph->box_w, glyph->box_w);
    }
    return buffer;
}

static int font_slot(uint32_t family, int px) {
    if (family < 2) return px == 25 ? 1 : px == 26 ? 2 : 0;
    return (family == 2 && px == 30) || (family == 3 && px == 40) ? 1 : 0;
}

void gallery_font(uint32_t font, int px, const uint8_t *data) {
    fonts[font][font_slot(font, px)] = (lv_font_t) {
        .get_glyph_dsc = describe_glyph,
        .get_glyph_bitmap = read_glyph,
        .line_height = data[0],
        .base_line = data[1],
        .underline_position = -1,
        .underline_thickness = 1,
        .dsc = data,
    };
}

static lv_text_align_t text_align(uint32_t align) {
    if (align == 3) return LV_TEXT_ALIGN_RIGHT;
    if (align == 2) return LV_TEXT_ALIGN_CENTER;
    return LV_TEXT_ALIGN_LEFT;
}

int gallery_apply_font(lv_obj_t *obj, uint32_t family, int px) {
    const lv_font_t *font = &fonts[family][font_slot(family, px)];
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
        case 29: lv_obj_set_style_bg_opa(obj, value, 0); break;
        case 45: lv_obj_set_style_clip_corner(obj, value, 0); break;
        case 48: lv_obj_set_style_border_width(obj, value, 0); break;
        case 49: lv_obj_set_style_border_color(obj, lv_color_hex(value), 0); break;
        case 50: lv_obj_set_style_border_opa(obj, value, 0); break;
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
        const value = localStorage.getItem("aeris:app:" + (document.querySelector('meta[name="aeris-storage"]')?.content || "") + ":" + UTF8ToString(key));
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
        localStorage.setItem("aeris:app:" + (document.querySelector('meta[name="aeris-storage"]')?.content || "") + ":" + UTF8ToString(key), btoa(bytes));
        return 0;
    } catch { return -5; }
});

EM_JS(int, storage_remove, (const char *key), {
    try {
        localStorage.removeItem("aeris:app:" + (document.querySelector('meta[name="aeris-storage"]')?.content || "") + ":" + UTF8ToString(key));
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

EMSCRIPTEN_KEEPALIVE void gallery_pointer(int x, int y, int phase) {
    if (!pointer) return;
    point = (lv_point_t){ x, y };
    if (phase == 3) {
        lv_indev_wait_release(pointer);
    }
    pressed = phase == 0 || phase == 1;
    lv_indev_read(pointer);
}

EM_JS(void, browser_panic, (const char *file, size_t length, unsigned line), {
    Module.galleryLog("error", new TextDecoder().decode(HEAPU8.subarray(file, file + length)) + ":" + line);
});

void gallery_panic(const char *file, size_t length, unsigned line) { browser_panic(file, length, line); }

EM_JS(void, inspect_node,
      (uintptr_t id, uintptr_t parent, int kind, const char *value, size_t length, int x, int y, int width, int height,
       int content_x, int content_y, int content_width, int content_height,
       int hidden, int clickable, int scrollable, int overflow, unsigned events,
       uint32_t background, uint32_t foreground, unsigned background_opacity, unsigned opacity,
       int radius, int border, int font_height, int alignment, unsigned checked, unsigned disabled), {
    Module.galleryNodes.push({
        id, parent, kind, value: value ? new TextDecoder().decode(HEAPU8.subarray(value, value + length)) : "",
        x, y, width, height, contentX: content_x, contentY: content_y,
        contentWidth: content_width, contentHeight: content_height,
        hidden: !!hidden, clickable: !!clickable, scrollable: !!scrollable, overflow: !!overflow, events,
        background: background >>> 0, foreground: foreground >>> 0, backgroundOpacity: background_opacity,
        opacity, radius, border, fontHeight: font_height, alignment, checked: !!checked, disabled: !!disabled,
    });
});

static void inspect_tree(lv_obj_t *obj, lv_obj_t *parent, bool hidden) {
    lv_area_t area, content;
    lv_obj_get_coords(obj, &area);
    lv_obj_get_content_coords(obj, &content);
    hidden = hidden || lv_obj_has_flag(obj, LV_OBJ_FLAG_HIDDEN);
    int kind = !parent ? 0 : lv_obj_has_class(obj, &lv_label_class) ? 1 :
        lv_obj_has_class(obj, &lv_image_class) ? 2 : 3;
    const char *value = kind == 1 ? lv_label_get_text(obj) : NULL;
    const lv_font_t *font = lv_obj_get_style_text_font(obj, LV_PART_MAIN);
    inspect_node((uintptr_t)obj, (uintptr_t)parent, kind, value, value ? strlen(value) : 0,
                 area.x1, area.y1, lv_area_get_width(&area), lv_area_get_height(&area),
                 content.x1, content.y1, lv_area_get_width(&content), lv_area_get_height(&content),
                 hidden, lv_obj_has_flag(obj, LV_OBJ_FLAG_CLICKABLE), lv_obj_has_flag(obj, LV_OBJ_FLAG_SCROLLABLE),
                 lv_obj_has_flag(obj, LV_OBJ_FLAG_OVERFLOW_VISIBLE), lv_obj_get_event_count(obj),
                 lv_color_to_u32(lv_obj_get_style_bg_color(obj, LV_PART_MAIN)) & 0xffffff,
                 lv_color_to_u32(lv_obj_get_style_text_color(obj, LV_PART_MAIN)) & 0xffffff,
                 lv_obj_get_style_bg_opa(obj, LV_PART_MAIN), lv_obj_get_style_opa_recursive(obj, LV_PART_MAIN),
                 lv_obj_get_style_radius(obj, LV_PART_MAIN), lv_obj_get_style_border_width(obj, LV_PART_MAIN),
                 font ? font->line_height : 0, lv_obj_get_style_text_align(obj, LV_PART_MAIN),
                 lv_obj_has_state(obj, LV_STATE_CHECKED), lv_obj_has_state(obj, LV_STATE_DISABLED));
    uint32_t count = lv_obj_get_child_count(obj);
    for (uint32_t index = 0; index < count; index++) {
        inspect_tree(lv_obj_get_child(obj, index), obj, hidden);
    }
}

EMSCRIPTEN_KEEPALIVE void gallery_inspect(void) {
    lv_obj_t *root = lv_screen_active();
    if (root) {
        lv_obj_update_layout(root);
        inspect_tree(root, NULL, false);
    }
}

static void flush_display(lv_display_t *display, const lv_area_t *area, uint8_t *pixels) {
    (void)area;
    canvas_paint(pixels, lv_display_get_horizontal_resolution(display),
                 lv_display_get_vertical_resolution(display));
    lv_display_flush_ready(display);
}

static void draw(void) { lv_timer_handler(); }

int main(void) {
    lv_init();
    lv_tick_set_cb(browser_ticks);
    lv_display_t *display = lv_display_create(WIDTH, HEIGHT);
    lv_display_set_color_format(display, LV_COLOR_FORMAT_ARGB8888);
    lv_display_set_buffers(display, framebuffer, NULL, sizeof(framebuffer), LV_DISPLAY_RENDER_MODE_FULL);
    lv_display_set_flush_cb(display, flush_display);

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
