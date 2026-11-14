EMCC ?= emcc
EMAR ?= emar
LVGL := lvgl
LVGL_SOURCES := $(shell find $(LVGL)/src -name '*.c')
LVGL_OBJECTS := $(patsubst $(LVGL)/src/%.c,objects/%.o,$(LVGL_SOURCES))
CFLAGS := -Os -flto -I$(LVGL) -DLV_CONF_SKIP -DLV_COLOR_DEPTH=32 \
	-DLV_USE_STDLIB_MALLOC=LV_STDLIB_CLIB -DLV_USE_THEME_DEFAULT=0 \
	-DLV_USE_THEME_SIMPLE=0 -DLV_USE_THEME_MONO=0 -DLV_USE_SNAPSHOT=1

.PHONY: all
all: liblvgl.a browser.o

liblvgl.a: $(LVGL_OBJECTS)
	"$(EMAR)" rcs $@ $^

objects/%.o: $(LVGL)/src/%.c Makefile
	@mkdir -p "$(@D)"
	"$(EMCC)" $(CFLAGS) -MMD -MP -c "$<" -o "$@"

browser.o: browser.c Makefile
	"$(EMCC)" $(CFLAGS) -MMD -MP -c "$<" -o "$@"

-include browser.d $(LVGL_OBJECTS:.o=.d)
