# Build the viewer. Needs raylib 5.0 or newer (checked with 5.5 and 6.0).
#
#   make                 build ./heron_brush-raylib-demo
#   make run             build and run it
#   make RAYLIB=/path    use a raylib built somewhere other than the system
#   make anim-check      build ./heron_brush-raylib-anim with motion_lab's animations and
#                        check them against the collision (src/anim_check.c)
#   make feature-check   check the feature lab export (src/verify_feature_lab.c)
#   make ANIM=path/to/map_animation.h
#                        play another map's editor animations (src/animated.h);
#                        the default is the demo map's, which has none
#
# RAYLIB= takes priority. It accepts either a git checkout (headers and
# libraylib.a in src/) or an unpacked release archive (include/ and lib/).
# Otherwise, if pkg-config knows raylib (`pkg-config --exists raylib`, e.g. a
# Homebrew or distro install), its flags are used. Failing both, plain
# -lraylib is tried. The platform libraries (OpenGL/X11 on Linux, frameworks on
# macOS) are added in every case.

BIN     := heron_brush-raylib-demo
SRC     := src/main.c src/level.c src/collision.c src/player.c src/movers.c src/game.c src/lighting.c src/glb.c src/animated.c src/anim_check.c
CFLAGS  += -std=c99 -O2 -Wall -Wextra -Isrc

# The editor animations compiled in: the C99 player the export writes for one map.
ANIM    ?= assets/maps/demo_animation.h
CFLAGS  += -DHERON_BRUSH_ANIMATION_HEADER='"$(abspath $(ANIM))"'

RAYLIB  ?=
PKG     := $(shell pkg-config --exists raylib 2>/dev/null && echo yes)

ifneq ($(RAYLIB),)
  RL_INC := $(if $(wildcard $(RAYLIB)/include/raylib.h),$(RAYLIB)/include,$(RAYLIB)/src)
  RL_LIB := $(if $(wildcard $(RAYLIB)/lib),$(RAYLIB)/lib,$(RAYLIB)/src)
  RL_A   := $(wildcard $(RL_LIB)/libraylib.a)
  CFLAGS += -I$(RL_INC)
  # Prefer the static lib if the tree has one, so a release archive links with
  # no rpath to sort out; fall back to -lraylib for a shared-only build.
  RL_LDFLAGS := $(if $(RL_A),$(RL_A),-L$(RL_LIB) -lraylib)
else ifeq ($(PKG),yes)
  CFLAGS += $(shell pkg-config --cflags raylib)
  RL_LDFLAGS := $(shell pkg-config --libs raylib)
else
  RL_LDFLAGS := -lraylib
endif

UNAME := $(shell uname -s)
ifeq ($(UNAME),Darwin)
  SYS_LDFLAGS := -framework OpenGL -framework Cocoa -framework IOKit \
                 -framework CoreVideo -framework CoreAudio -lm
else
  SYS_LDFLAGS := -lGL -lm -lpthread -ldl -lrt -lX11
endif
LDFLAGS += $(RL_LDFLAGS) $(SYS_LDFLAGS)

$(BIN): $(SRC) $(wildcard src/*.h) $(ANIM)
	$(CC) $(CFLAGS) $(SRC) -o $@ $(LDFLAGS)

run: $(BIN)
	./$(BIN)

# The editor's animations and the collision together, on demos/motion_lab as exported
# by export_motion_lab.py: the same checks and limits as its Godot project's verify.gd.
anim-check:
	$(MAKE) -B BIN=heron_brush-raylib-anim ANIM=assets/maps/motion_lab_animation.h RAYLIB=$(RAYLIB)
	./heron_brush-raylib-anim --level assets/maps/motion_lab.level --anim-check

clean:
	rm -f $(BIN) heron_brush-raylib-anim heron_brush-raylib.png

game-check:
	$(CC) $(CFLAGS) src/verify_game.c src/level.c src/collision.c src/player.c src/movers.c src/game.c src/animated.c -o /tmp/heron-raylib-game-check $(LDFLAGS)
	/tmp/heron-raylib-game-check

# The feature lab, exported by export_feature_lab.py, with the editor's poses from
# feature_lab_poses.py --text: `make feature-check FEATURE=/tmp/heron-feature-raylib
# POSES=/tmp/poses.txt [SHOT=/tmp/raylib.png]`. Needs an OpenGL context for its GPU half.
FEATURE ?= /tmp/heron-feature-raylib
POSES   ?= /tmp/feature-lab-poses.txt
SHOT    ?=
feature-check:
	$(CC) $(CFLAGS) -I$(FEATURE) -UHERON_BRUSH_ANIMATION_HEADER -DHERON_BRUSH_ANIMATION_HEADER='"feature_lab_animation.h"' \
		src/verify_feature_lab.c src/level.c src/collision.c src/animated.c -o /tmp/heron-raylib-feature-check $(LDFLAGS)
	cd $(FEATURE) && /tmp/heron-raylib-feature-check $(abspath $(POSES)) $(if $(SHOT),$(abspath $(SHOT)))

.PHONY: run clean anim-check game-check feature-check
