# Rendering (`src/render/`)

Sprite rendering on top of the **egui painter**. `RenderEngine::render(scene)`
turns a scene into a z-sorted queue of draw commands in viewport-relative
screen space; callers paint them with `paint_sprites`, using GPU textures from
`get_egui_texture`. Both the editor viewport and the runtime work this way.

## Key pieces

| Item | Responsibility |
|---|---|
| `Camera` | `position` = world point at the viewport's top-left, `zoom` = screen pixels per world unit; `world_to_screen` and its inverse `screen_to_world` |
| `RenderEngine` | Owns a `Camera`, the viewport size, and caches of decoded images and GPU textures keyed by file path (`path_to_uuid`); `get_grid_lines` draws the editor grid, whose cell size `set_grid_size` sets (`DEFAULT_GRID_SIZE`, 32) |
| `RenderQueueEntry` | One sprite: entity, texture, screen position and size (`screen_rect`), texture rectangle, tint, rotation (radians), z; or a tilemap's or emitter's textured `quads` |
| `paint_sprites` | Paints queue entries in order as few meshes as possible: consecutive entries with the same texture share one mesh, which egui draws in one call |
| `paint_sprite` | Paints a single textured rect, rotated around its center if needed |
| `game_view(scene, viewport_size)` | Fits the scene's default camera into a viewport (see below) |
| `render_colliders` / `paint_collider_outline` | Debug outlines of physics colliders, rotated with their bodies: circles red, rectangles blue, sensors (trigger zones) green, one-way platforms the top edge in cyan |

## Sprites

```mermaid
graph TD
    S[Scene entities] -->|"first image, x/y/z, sprite attributes"| R["render()"]
    R -->|"decode once per path"| TC["decoded RGBA cache"]
    R -->|"cull to viewport, sort by z"| Q[render queue]
    Q --> P["caller: paint_sprites()"]
    P -->|"get_egui_texture(): upload once"| GT["GPU textures"]
```

- An entity's sprite is its **first** image; `x`/`y` is the sprite's top-left
  corner. Its size is the image's pixel size times the optional `scale`
  Vector2 attribute times the camera zoom.
- Optional attributes change how the sprite is drawn, in the editor and in
  games:

  | Attribute | Type | Effect |
  |---|---|---|
  | `rotation` | Float (degrees) | Rotates around the sprite's center (the physics engine writes it for rotating bodies) |
  | `scale` | Vector2 | Multiplies the size |
  | `flip_x` / `flip_y` | Boolean | Mirrors the image horizontally / vertically (e.g. a character facing left) |
  | `opacity` | Float | 1 is opaque, 0 invisible; values outside 0–1 are clamped |
  | `tint` | String | `"#rrggbb"` or `"#rrggbbaa"`: multiplies the image's colors (white leaves it unchanged); an invalid value is reported once and ignored |

  A value of another type is ignored. Scripts change them with
  `set_attribute` once the entity has them (added in the Inspector, or with
  `create_attribute_*`).
- Lower `z` is drawn first. The sort is total, so a NaN `z` can't break it.
- Images that fail to load are skipped and reported once in the console; they
  are retried only after the cache is cleared.
- Texture ids derive from image paths (`path_to_uuid`, a hash); the engine
  caches them per path, so a frame doesn't hash every sprite's path again.
- Images larger than the GPU's maximum texture size are downscaled on upload
  (they still cover their full size on screen).

## Sprite sheets and animation

An entity with a `frame_size` (Vector2, in pixels) treats its image as a
**sprite sheet**: a grid of frames of that size, numbered from 0 left to
right, then top to bottom. Its `frame` attribute (Integer, or Float) selects
the frame shown; numbers wrap around, and the sprite (and its physics body)
is one frame big. `sheet_frame` computes a frame's texture rectangle.

Animations play frames in order while the game runs. They are driven by
attributes, so they can be set up in the Inspector or from scripts
(`play_animation`, see [scripting.md](scripting.md)):

| Attribute | Type | Default | Meaning |
|---|---|---|---|
| `animation` | String | — | Frames to play: numbers and inclusive ranges separated by commas, e.g. `"0-3"`, `"0,1,2,1"` or `"7-4"` |
| `animation_fps` | Float | 10 | Frames per second |
| `animation_loop` | Boolean | true | Start over at the end; otherwise stop on the last frame |
| `animation_time` | Float | 0 | Seconds into the animation; advanced by the runtime, reset to start over |

`advance_animations(scene, dt)`, called by the runtime once per frame,
advances every entity with a non-empty `animation` and writes its `frame`
(created as an Integer if missing; an existing Float stays a Float). A
looping animation's time wraps at the end of each cycle. Invalid `animation`
values are reported once per Play and leave the entity unchanged. The editor
doesn't play animations: it shows the current `frame`.

## Tilemaps

An entity with a tilemap (see [scene.md](scene.md#tilemaps)) is drawn as its
tiles instead of a sprite: `render()` makes one queue entry for the map, whose
`tiles` list the visible cells as `TileQuad`s (screen rect and tileset
texture rectangle), and whose `screen_pos`/`screen_size` cover the whole grid
(for picking in the editor). Only the cells inside the viewport are
visited, and `paint_sprites` draws them all in one mesh, so a map's cost
follows what's on screen, not its size. `z`, `tint` and `opacity` apply to
the map; `rotation`, `scale` and mirroring don't.

Each tile's texture rectangle is inset by half a texel, so linear filtering
doesn't blend in the neighbouring tiles of the tileset (visible seams).

## Particles

An entity with a `particle_rate` attribute is a **particle emitter**: it
gives off small, short-lived sprites (sparks, smoke, dust, rain), and isn't
drawn itself. Its first image, if it has one, is the particles' texture;
otherwise they use a built-in soft white dot (`PARTICLE_TEXTURE_ID`). The
editor's Add Feature → Effects → Particles adds every attribute with working
values (warm sparks); `ParticleSettings` reads them, keeping the default for
a missing or invalid one (reported once in the console):

| Attribute | Type | Meaning |
|---|---|---|
| `particle_rate` | Float | Particles per second; 0 gives off only bursts |
| `particle_lifetime` | Float | Seconds a particle lives |
| `particle_velocity` | Vector2 | A new particle's velocity, px/s (+y is down) |
| `particle_spread` | Float | Degrees the direction varies by around `particle_velocity`; 360 sends them every way |
| `particle_variation` | Float | 0 to 1: how much slower and shorter-lived a particle may randomly be |
| `particle_gravity` | Vector2 | Acceleration of the particles, px/s² |
| `particle_size`, `particle_end_size` | Float | Width and height when born and when dying, in world units |
| `particle_color`, `particle_end_color` | String | Colors when born and when dying (`"#rrggbb"` or `"#rrggbbaa"`), blended before applying opacity, so fading out to `#ff000000` also turns the color red |
| `particle_area` | Vector2 | Width and height of the rectangle particles start in, from the emitter's `x`/`y`; (0, 0) is that point |

The particles live in the `RenderEngine`, in world space: once given off,
they don't follow the emitter. `update_particles(scene, dt, emitting,
bursts)` moves and ages them, lets the emitters for which `emitting` holds
give off particles at their rate, and adds `bursts` (`ParticleBurst`, e.g.
from scripts' `emit_particles`). A burst carries its emitter's settings, so
it plays out even when the emitter is removed in the same frame (an
explosion where an enemy was); the particles of an emitter that is gone live
out their lives. `render()` makes one queue entry per emitter: its particles
inside the viewport as `quads`, drawn in one mesh with the others of the same
texture, and a marker square around its position (`EMITTER_MARKER_SIZE`) for
the editor. At most `MAX_PARTICLES` (20,000) live at once, and a burst gives
off at most `MAX_BURST` (1,000).

## Game camera

`camera_frame(scene)` describes the frame the scene's default camera entity
shows:

- The camera entity's `x`/`y` is the **center** of its frame; the frame shows
  `width / zoom` × `height / zoom` world units (`width`, `height` and `zoom`
  are attributes of the camera entity): `world_min` and `world_size`.
- `size` is the camera's `width` × `height`: the frame's size in its own
  pixels, whatever the zoom. Text drawn over the game is laid out in this
  space; `world_to_pixels` converts a world position into it.

`game_view(scene, viewport_size)` fits that frame into a viewport: scaled
uniformly and centered; the returned `frame` rect is the part of the viewport
it covers (the rest is letterboxing). The runtime uses it for exported games;
the editor draws the same frame as a red rectangle (`get_game_camera_bounds`).

## Text

`HudText` is one text drawn over the game: its string, position (the top of
the text, in camera frame pixels), font height, color and horizontal
alignment (`TextAlign`). `paint_texts(painter, texts, origin, scale)` draws
them with egui's built-in fonts, given the screen position of the camera
frame's top-left corner and the screen pixels per frame pixel, so text scales
with the game. Texts smaller than one screen pixel are skipped.

`parse_color("#rrggbb")` / `parse_color("#rrggbbaa")` reads the color strings
scripts use.

## Caches

`clear_cache()` drops decoded images, GPU textures and remembered load
failures; `unload_texture(path)` does so for one file; `clear_particles()`
removes every particle; `cleanup()` does all of it and resets the camera. Images are not reloaded automatically when their file changes on
disk.

## Known limitations

- The camera entity's `rotation` attribute is not applied.
- Text uses egui's built-in fonts (plus system fonts for missing glyphs);
  projects can't bring their own, and text can't be measured from scripts.
- Text is always drawn over the sprites, in camera frame space; there is no
  text placed in the world (e.g. a label following an entity).
- Sprite sheets are uniform grids; frames of different sizes (texture atlases)
  and spacing between frames aren't supported. The same goes for tilesets,
  and tiles can't be animated.
- Particles don't collide, rotate or animate their texture, and every
  particle of an emitter has the same texture; an emitter gives off
  particles only into the world (none follow it), and only from a point or
  a rectangle.
- The same image referenced through two different path spellings is cached
  twice.
- The editor and the runtime each own a `RenderEngine`, so images used in
  both are decoded and uploaded twice.
