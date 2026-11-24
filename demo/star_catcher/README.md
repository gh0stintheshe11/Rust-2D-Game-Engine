# Star Catcher

A small demo game for the Rust 2D Game Engine. Stars and rocks fall from the
sky: catch the stars with the paddle, and don't let a rock hit it. Drops come
faster over time.

![Star Catcher](../../docs/images/star_catcher.png)

**Controls:** ← / → or A / D. After a game over, Enter or R starts a new game.

## Running it

- **In the editor:** File → Open Project → this folder, then ▶ Play.
- **As a standalone game:** `cargo run -p star_catcher` from the repository
  root, or Project → Build Project in the editor (the game ends up in
  `target/release/`).

## What it shows

| Feature | Where |
|---|---|
| Kinematic body moved by a script, tuned by an attribute (`speed`) | `paddle` entity, `paddle.lua` |
| Spawning physics entities at runtime: image, script, collider shape, spin | `spawner.lua` |
| `on_collision`, removing entities, sounds, `end_game` | `falling.lua` |
| Explicit collider sizes on entities without images (the side walls) | `left_wall`, `right_wall` |
| Global gravity from a script, random seeds from an attribute | `spawner.lua` (`seed` = 0 is random) |
| Camera framing: a 480 × 640 frame centered on the camera entity | `main_camera` |

## Assets

All images and sounds in `assets/` were generated for this project and are
covered by the repository's license (Apache-2.0).
