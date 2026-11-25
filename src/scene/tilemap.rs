//! Tilemaps: a grid of tiles drawn from one tileset image, attached to an
//! entity.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Most columns, or rows, a tilemap can span.
pub const MAX_TILEMAP_SIDE: u32 = 1024;

/// Cells' columns and rows go from -`MAX_TILEMAP_COORDINATE` to
/// `MAX_TILEMAP_COORDINATE` (about a million): farther out, world positions
/// lose precision, and physics' voxel coordinates would overflow.
pub const MAX_TILEMAP_COORDINATE: i32 = 1 << 20;

/// Whether a column or row is one a cell can have.
fn in_range(coordinate: i64) -> bool {
    coordinate.abs() <= i64::from(MAX_TILEMAP_COORDINATE)
}

/// Most frames a tile animation has.
pub const MAX_TILE_ANIMATION_FRAMES: usize = 4096;

/// A tile of the tileset shown as other tiles in turn (water, torches,
/// grass in the wind). Cells keep the tile, which decides how they collide
/// and what scripts read; only the picture changes.
#[derive(Clone, Debug, PartialEq)]
pub struct TileAnimation {
    frames: Vec<u32>,
    fps: f32,
}

impl TileAnimation {
    /// Show `frames` (tile numbers) in turn, `fps` per second, looping.
    pub fn new(frames: Vec<u32>, fps: f32) -> Result<Self, String> {
        if frames.is_empty() || frames.len() > MAX_TILE_ANIMATION_FRAMES {
            return Err(format!(
                "a tile animation has 1 to {} frames, not {}",
                MAX_TILE_ANIMATION_FRAMES,
                frames.len()
            ));
        }
        if !(fps.is_finite() && fps > 0.0) {
            return Err(format!(
                "a tile animation's fps must be above 0, not {}",
                fps
            ));
        }
        Ok(Self { frames, fps })
    }

    pub fn frames(&self) -> &[u32] {
        &self.frames
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// The tile shown `seconds` into the animation.
    pub fn frame_at(&self, seconds: f32) -> u32 {
        // (The float-to-int conversion saturates, and NaN becomes 0)
        let index = (seconds.max(0.0) * self.fps) as usize % self.frames.len();
        self.frames[index]
    }
}

/// How the cells with a tile collide, in a tilemap with `has_collision`.
/// Set per tile of the tileset, like the tile's picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileCollision {
    /// Solid from every side: ground, walls, ceilings
    #[default]
    Solid,
    /// Solid only from above (a one-way platform): bodies jump up through
    /// it and land on it
    OneWay,
    /// Not solid: decoration
    Passable,
}

/// A grid of tiles drawn from one tileset image. Cell (0, 0) has its
/// top-left corner at the entity's `x`/`y`; columns grow to the right and
/// rows downwards, and cells can have negative coordinates (within
/// ±[`MAX_TILEMAP_COORDINATE`]).
///
/// The grid grows to include every cell that gets a tile (up to
/// [`MAX_TILEMAP_SIDE`] columns and rows); scene files store only the area
/// that has tiles.
#[derive(Clone, Debug, PartialEq)]
pub struct Tilemap {
    /// The tileset: an image cut into a grid of `tile_size` cells, numbered
    /// from 0 left to right, then top to bottom (like a sprite sheet).
    pub tileset: PathBuf,
    /// A tile's width and height, in tileset pixels and in world units.
    pub tile_size: (u32, u32),
    /// Cell coordinates of the grid's first column and row
    origin: (i32, i32),
    /// Columns and rows
    size: (u32, u32),
    /// Row-major cells: 0 is empty, n is tile n - 1
    cells: Vec<u32>,
    /// How tiles of the tileset collide, for those that aren't solid
    collision: BTreeMap<u32, TileCollision>,
    /// Tiles of the tileset that are animated
    animations: BTreeMap<u32, TileAnimation>,
}

impl Tilemap {
    /// An empty tilemap.
    pub fn new(tileset: PathBuf, tile_size: (u32, u32)) -> Self {
        Self {
            tileset,
            tile_size: (tile_size.0.max(1), tile_size.1.max(1)),
            origin: (0, 0),
            size: (0, 0),
            cells: Vec::new(),
            collision: BTreeMap::new(),
            animations: BTreeMap::new(),
        }
    }

    /// How a tile of the tileset is animated, if it is.
    pub fn tile_animation(&self, tile: u32) -> Option<&TileAnimation> {
        self.animations.get(&tile)
    }

    /// Animate a tile of the tileset, or with `None` stop animating it.
    pub fn set_tile_animation(&mut self, tile: u32, animation: Option<TileAnimation>) {
        match animation {
            Some(animation) => self.animations.insert(tile, animation),
            None => self.animations.remove(&tile),
        };
    }

    /// The animated tiles of the tileset, in tile order.
    pub fn tile_animations(&self) -> impl Iterator<Item = (u32, &TileAnimation)> {
        self.animations
            .iter()
            .map(|(tile, animation)| (*tile, animation))
    }

    /// Whether any tile of the tileset is animated.
    pub fn has_animations(&self) -> bool {
        !self.animations.is_empty()
    }

    /// The tile a cell with `tile` shows `seconds` into its animation:
    /// `tile` itself unless it's animated.
    pub fn shown_tile(&self, tile: u32, seconds: f32) -> u32 {
        self.animations
            .get(&tile)
            .map_or(tile, |animation| animation.frame_at(seconds))
    }

    /// How the cells with this tile of the tileset collide.
    pub fn tile_collision(&self, tile: u32) -> TileCollision {
        self.collision.get(&tile).copied().unwrap_or_default()
    }

    /// Set how the cells with this tile of the tileset collide.
    pub fn set_tile_collision(&mut self, tile: u32, collision: TileCollision) {
        if collision == TileCollision::Solid {
            self.collision.remove(&tile);
        } else {
            self.collision.insert(tile, collision);
        }
    }

    /// The tile at a cell, if it has one.
    pub fn get(&self, column: i32, row: i32) -> Option<u32> {
        let index = self.index(column, row)?;
        self.cells[index].checked_sub(1)
    }

    /// Put `tile` in a cell, or with `None` empty it; returns whether the
    /// cell changed. The grid grows as needed; a tilemap can't span more
    /// than [`MAX_TILEMAP_SIDE`] columns or rows, and cells' columns and
    /// rows are within ±[`MAX_TILEMAP_COORDINATE`].
    pub fn set(&mut self, column: i32, row: i32, tile: Option<u32>) -> Result<bool, String> {
        let stored = stored(tile)?;
        let index = match self.index(column, row) {
            Some(index) => index,
            // Emptying a cell outside the grid changes nothing
            None if stored == 0 => return Ok(false),
            None => {
                self.include(column, row)?;
                self.index(column, row)
                    .expect("the grid now includes the cell")
            }
        };
        let changed = self.cells[index] != stored;
        self.cells[index] = stored;
        Ok(changed)
    }

    /// Put `tile` in every cell from `columns.0` to `columns.1` and from
    /// `rows.0` to `rows.1` (inclusive, either way round), or with `None`
    /// empty them; returns the cells that changed. The grid grows as for
    /// [`set`](Self::set), and a rectangle it can't take changes nothing.
    pub fn fill_rect(
        &mut self,
        columns: (i32, i32),
        rows: (i32, i32),
        tile: Option<u32>,
    ) -> Result<Vec<(i32, i32)>, String> {
        let stored = stored(tile)?;
        let mut first = (columns.0.min(columns.1), rows.0.min(rows.1));
        let mut last = (columns.0.max(columns.1), rows.0.max(rows.1));
        if stored != 0 {
            self.include_area(first, last)?;
        } else {
            // Emptying: only cells of the grid can change
            let Some((origin, size)) = self.grid_bounds() else {
                return Ok(Vec::new());
            };
            let end = (
                (i64::from(origin.0) + i64::from(size.0) - 1) as i32,
                (i64::from(origin.1) + i64::from(size.1) - 1) as i32,
            );
            first = (first.0.max(origin.0), first.1.max(origin.1));
            last = (last.0.min(end.0), last.1.min(end.1));
        }
        let mut changed = Vec::new();
        for row in first.1..=last.1 {
            for column in first.0..=last.0 {
                if let Some(index) = self.index(column, row)
                    && self.cells[index] != stored
                {
                    self.cells[index] = stored;
                    changed.push((column, row));
                }
            }
        }
        Ok(changed)
    }

    /// Put `tile` (or with `None`, nothing) in the cell and every cell
    /// connected to it through sides, not corners, that holds what it holds,
    /// within [`bounds`](Self::bounds): a paint bucket. Returns the cells
    /// that changed; none when the cell is beyond the bounds or holds that
    /// already.
    pub fn flood_fill(
        &mut self,
        column: i32,
        row: i32,
        tile: Option<u32>,
    ) -> Result<Vec<(i32, i32)>, String> {
        let stored = stored(tile)?;
        let Some((origin, size)) = self.bounds() else {
            return Ok(Vec::new());
        };
        let (width, height) = (i64::from(size.0), i64::from(size.1));
        // A cell's place in the bounds, if it's in them
        let place = |column: i32, row: i32| {
            let x = i64::from(column) - i64::from(origin.0);
            let y = i64::from(row) - i64::from(origin.1);
            (x >= 0 && y >= 0 && x < width && y < height).then(|| (y * width + x) as usize)
        };
        let Some(start) = place(column, row) else {
            return Ok(Vec::new());
        };
        let target = self.get(column, row).map_or(0, |tile| tile + 1);
        if target == stored {
            return Ok(Vec::new());
        }
        let mut seen = vec![false; (width * height) as usize];
        seen[start] = true;
        let mut pending = vec![(column, row)];
        let mut changed = Vec::new();
        while let Some((column, row)) = pending.pop() {
            let Some(index) = self.index(column, row) else {
                continue;
            };
            if self.cells[index] != target {
                continue;
            }
            self.cells[index] = stored;
            changed.push((column, row));
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (next_column, next_row) = (column.wrapping_add(dx), row.wrapping_add(dy));
                if let Some(next) = place(next_column, next_row)
                    && !seen[next]
                {
                    seen[next] = true;
                    pending.push((next_column, next_row));
                }
            }
        }
        Ok(changed)
    }

    /// Every cell with a tile: `((column, row), tile)`, row by row.
    pub fn tiles(&self) -> impl Iterator<Item = ((i32, i32), u32)> + '_ {
        let (columns, _) = self.size;
        self.cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| **cell != 0)
            .map(move |(index, cell)| {
                let index = index as i64;
                let column = self.origin.0 as i64 + index % columns as i64;
                let row = self.origin.1 as i64 + index / columns as i64;
                ((column as i32, row as i32), cell - 1)
            })
    }

    /// The tiles of the cells in `columns` × `rows` (both inclusive ranges
    /// of cell coordinates), row by row. Cheaper than [`tiles`](Self::tiles)
    /// when only a part of the map is visible.
    pub fn tiles_in(
        &self,
        columns: (i32, i32),
        rows: (i32, i32),
    ) -> impl Iterator<Item = ((i32, i32), u32)> + '_ {
        // The part of a range within the grid (in i64, like the offsets
        // below, so no sum overflows)
        let within = |range: (i32, i32), origin: i32, size: u32| {
            let first = i64::from(range.0).max(i64::from(origin));
            let last = i64::from(range.1).min(i64::from(origin) + i64::from(size) - 1);
            (first, last)
        };
        let (first_column, last_column) = within(columns, self.origin.0, self.size.0);
        let (first_row, last_row) = within(rows, self.origin.1, self.size.1);
        // Each row's cells in range as one slice: no index math or bounds
        // check per cell (this runs for every visible cell, every frame)
        let width = self.size.0 as usize;
        let start = (first_column - i64::from(self.origin.0)).max(0) as usize;
        let count = (last_column - first_column + 1).max(0) as usize;
        // No column in range (the range is beside the grid): no row either,
        // since `start` may then be past the row's end
        let last_row = if count == 0 { first_row - 1 } else { last_row };
        (first_row..=last_row).flat_map(move |row| {
            let row_start = (row - i64::from(self.origin.1)) as usize * width + start;
            self.cells[row_start..row_start + count]
                .iter()
                .enumerate()
                .filter(|(_, cell)| **cell != 0)
                .map(move |(offset, cell)| {
                    let column = first_column + offset as i64;
                    ((column as i32, row as i32), cell - 1)
                })
        })
    }

    /// At least as many as the cells with a tile: the grid's size, which
    /// unlike [`tile_count`](Self::tile_count) costs nothing to compute.
    pub fn tile_count_bound(&self) -> i64 {
        self.size.0 as i64 * self.size.1 as i64
    }

    /// How many cells have a tile.
    pub fn tile_count(&self) -> usize {
        self.cells.iter().filter(|cell| **cell != 0).count()
    }

    /// The cells with tiles span this area: `(first column, first row)` and
    /// `(columns, rows)`; `None` without tiles.
    pub fn bounds(&self) -> Option<((i32, i32), (u32, u32))> {
        let mut tiles = self.tiles();
        let ((column, row), _) = tiles.next()?;
        let (mut min, mut max) = ((column, row), (column, row));
        for ((column, row), _) in tiles {
            min = (min.0.min(column), min.1.min(row));
            max = (max.0.max(column), max.1.max(row));
        }
        let size = ((max.0 - min.0 + 1) as u32, (max.1 - min.1 + 1) as u32);
        Some((min, size))
    }

    /// The grid the map has grown to: `(first column, first row)` and
    /// `(columns, rows)`, at least as large as [`bounds`](Self::bounds) (it
    /// doesn't shrink when tiles are erased). Unlike `bounds`, it costs
    /// nothing to compute.
    pub fn grid_bounds(&self) -> Option<((i32, i32), (u32, u32))> {
        (!self.cells.is_empty()).then_some((self.origin, self.size))
    }

    /// Remove every tile.
    pub fn clear(&mut self) {
        self.origin = (0, 0);
        self.size = (0, 0);
        self.cells.clear();
    }

    /// The cell containing a world position, for a map whose cell (0, 0)
    /// has its top-left corner at `map_position` (the entity's `x`/`y`).
    pub fn cell_at(&self, map_position: (f32, f32), world: (f32, f32)) -> (i32, i32) {
        let (width, height) = (self.tile_size.0 as f32, self.tile_size.1 as f32);
        let column = ((world.0 - map_position.0) / width).floor();
        let row = ((world.1 - map_position.1) / height).floor();
        (saturate(column), saturate(row))
    }

    fn index(&self, column: i32, row: i32) -> Option<usize> {
        let x = column as i64 - self.origin.0 as i64;
        let y = row as i64 - self.origin.1 as i64;
        let (columns, rows) = (self.size.0 as i64, self.size.1 as i64);
        (x >= 0 && y >= 0 && x < columns && y < rows).then(|| (y * columns + x) as usize)
    }

    /// Grow the grid to include a cell, keeping the tiles where they are.
    fn include(&mut self, column: i32, row: i32) -> Result<(), String> {
        self.include_area((column, row), (column, row))
    }

    /// Grow the grid to include the cells from `first` to `last` (columns
    /// and rows, `first` the smaller), keeping the tiles where they are, or
    /// change nothing when it can't.
    fn include_area(&mut self, first: (i32, i32), last: (i32, i32)) -> Result<(), String> {
        for (column, row) in [first, last] {
            if !(in_range(column.into()) && in_range(row.into())) {
                return Err(format!(
                    "cell {}, {} is out of range: columns and rows go from -{} to {}",
                    column, row, MAX_TILEMAP_COORDINATE, MAX_TILEMAP_COORDINATE
                ));
            }
        }
        let (first, last) = (
            (i64::from(first.0), i64::from(first.1)),
            (i64::from(last.0), i64::from(last.1)),
        );
        let (min, max) = if self.cells.is_empty() {
            (first, last)
        } else {
            let (origin_x, origin_y) = (self.origin.0 as i64, self.origin.1 as i64);
            (
                (origin_x.min(first.0), origin_y.min(first.1)),
                (
                    (origin_x + self.size.0 as i64 - 1).max(last.0),
                    (origin_y + self.size.1 as i64 - 1).max(last.1),
                ),
            )
        };
        let (columns, rows) = (max.0 - min.0 + 1, max.1 - min.1 + 1);
        if columns > MAX_TILEMAP_SIDE as i64 || rows > MAX_TILEMAP_SIDE as i64 {
            let cell = if (first.0, first.1) == (min.0, min.1) {
                last
            } else {
                first
            };
            return Err(format!(
                "a tilemap can't span more than {} columns or rows (cell {}, {})",
                MAX_TILEMAP_SIDE, cell.0, cell.1
            ));
        }
        if (min, (columns, rows))
            == (
                (self.origin.0 as i64, self.origin.1 as i64),
                (self.size.0 as i64, self.size.1 as i64),
            )
        {
            return Ok(());
        }
        let mut cells = vec![0; (columns * rows) as usize];
        for ((x, y), tile) in self.tiles() {
            let index = (y as i64 - min.1) * columns + (x as i64 - min.0);
            cells[index as usize] = tile + 1;
        }
        self.origin = (min.0 as i32, min.1 as i32);
        self.size = (columns as u32, rows as u32);
        self.cells = cells;
        Ok(())
    }
}

/// How a cell stores `tile`: 0 for none, the tile + 1 otherwise.
fn stored(tile: Option<u32>) -> Result<u32, String> {
    match tile {
        Some(tile) => tile
            .checked_add(1)
            .ok_or_else(|| format!("tile {} is too large", tile)),
        None => Ok(0),
    }
}

fn saturate(value: f32) -> i32 {
    // Float-to-int conversion saturates (and NaN becomes 0)
    value as i32
}

/// How a tilemap is stored in scene files: the area with tiles, one string
/// per row with a token per cell, `.` for an empty cell and otherwise the
/// tile number.
#[derive(Serialize, Deserialize)]
struct StoredTilemap {
    tileset: PathBuf,
    tile_size: (u32, u32),
    /// Tiles that aren't solid
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    collision: BTreeMap<u32, TileCollision>,
    /// Animated tiles
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    animations: BTreeMap<u32, StoredAnimation>,
    origin: (i32, i32),
    rows: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredAnimation {
    frames: Vec<u32>,
    fps: f32,
}

impl Serialize for Tilemap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (origin, (columns, rows)) = self.bounds().unwrap_or(((0, 0), (0, 0)));
        let rows = (0..rows as i32)
            .map(|y| {
                (0..columns as i32)
                    .map(|x| match self.get(origin.0 + x, origin.1 + y) {
                        Some(tile) => tile.to_string(),
                        None => ".".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        StoredTilemap {
            tileset: self.tileset.clone(),
            tile_size: self.tile_size,
            collision: self.collision.clone(),
            animations: self
                .animations
                .iter()
                .map(|(tile, animation)| {
                    let stored = StoredAnimation {
                        frames: animation.frames.clone(),
                        fps: animation.fps,
                    };
                    (*tile, stored)
                })
                .collect(),
            origin,
            rows,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Tilemap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        let stored = StoredTilemap::deserialize(deserializer)?;
        let rows: Vec<Vec<&str>> = stored
            .rows
            .iter()
            .map(|row| row.split_whitespace().collect())
            .collect();
        let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
        if columns > MAX_TILEMAP_SIDE as usize || rows.len() > MAX_TILEMAP_SIDE as usize {
            return Err(D::Error::custom(format!(
                "a tilemap can't span more than {} columns or rows",
                MAX_TILEMAP_SIDE
            )));
        }
        // The first and the last cell must be in range
        for (count, first) in [(columns, stored.origin.0), (rows.len(), stored.origin.1)] {
            let last = i64::from(first) + count as i64 - 1;
            if count > 0 && !(in_range(first.into()) && in_range(last)) {
                return Err(D::Error::custom(format!(
                    "tilemap cells out of range: columns and rows go from -{} to {}",
                    MAX_TILEMAP_COORDINATE, MAX_TILEMAP_COORDINATE
                )));
            }
        }

        // One allocation for the whole grid
        let mut cells = vec![0; columns * rows.len()];
        for (y, row) in rows.iter().enumerate() {
            for (x, token) in row.iter().enumerate() {
                if *token == "." {
                    continue;
                }
                let tile: u32 = token.parse().map_err(|_| {
                    D::Error::custom(format!("invalid tile {:?} in tilemap row {}", token, y))
                })?;
                cells[y * columns + x] = tile
                    .checked_add(1)
                    .ok_or_else(|| D::Error::custom(format!("tile {} is too large", tile)))?;
            }
        }
        let mut tilemap = Tilemap::new(stored.tileset, stored.tile_size);
        for (tile, collision) in stored.collision {
            tilemap.set_tile_collision(tile, collision);
        }
        for (tile, animation) in stored.animations {
            let animation = TileAnimation::new(animation.frames, animation.fps)
                .map_err(|e| D::Error::custom(format!("tile {}: {}", tile, e)))?;
            tilemap.set_tile_animation(tile, Some(animation));
        }
        if !cells.is_empty() {
            tilemap.origin = stored.origin;
            tilemap.size = (columns as u32, rows.len() as u32);
            tilemap.cells = cells;
        }
        Ok(tilemap)
    }
}
