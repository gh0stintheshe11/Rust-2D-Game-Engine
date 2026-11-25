//! Tilemaps: a grid of tiles drawn from one tileset image, attached to an
//! entity.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

/// Most columns, or rows, a tilemap can span.
pub const MAX_TILEMAP_SIDE: u32 = 1024;

/// A grid of tiles drawn from one tileset image. Cell (0, 0) has its
/// top-left corner at the entity's `x`/`y`; columns grow to the right and
/// rows downwards, and cells can have negative coordinates.
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
        }
    }

    /// The tile at a cell, if it has one.
    pub fn get(&self, column: i32, row: i32) -> Option<u32> {
        let index = self.index(column, row)?;
        self.cells[index].checked_sub(1)
    }

    /// Put `tile` in a cell, or with `None` empty it; returns whether the
    /// cell changed. The grid grows as needed; a tilemap can't span more
    /// than [`MAX_TILEMAP_SIDE`] columns or rows.
    pub fn set(&mut self, column: i32, row: i32, tile: Option<u32>) -> Result<bool, String> {
        let stored = match tile {
            Some(tile) => tile
                .checked_add(1)
                .ok_or_else(|| format!("tile {} is too large", tile))?,
            None => 0,
        };
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
        let (first_column, last_column) = (
            columns.0.max(self.origin.0),
            columns.1.min(self.origin.0 + self.size.0 as i32 - 1),
        );
        let (first_row, last_row) = (
            rows.0.max(self.origin.1),
            rows.1.min(self.origin.1 + self.size.1 as i32 - 1),
        );
        (first_row..=last_row).flat_map(move |row| {
            (first_column..=last_column)
                .filter_map(move |column| self.get(column, row).map(|tile| ((column, row), tile)))
        })
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
        let (min, max) = if self.cells.is_empty() {
            ((column as i64, row as i64), (column as i64, row as i64))
        } else {
            let (origin_x, origin_y) = (self.origin.0 as i64, self.origin.1 as i64);
            (
                (origin_x.min(column as i64), origin_y.min(row as i64)),
                (
                    (origin_x + self.size.0 as i64 - 1).max(column as i64),
                    (origin_y + self.size.1 as i64 - 1).max(row as i64),
                ),
            )
        };
        let (columns, rows) = (max.0 - min.0 + 1, max.1 - min.1 + 1);
        if columns > MAX_TILEMAP_SIDE as i64 || rows > MAX_TILEMAP_SIDE as i64 {
            return Err(format!(
                "a tilemap can't span more than {} columns or rows (cell {}, {})",
                MAX_TILEMAP_SIDE, column, row
            ));
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
    origin: (i32, i32),
    rows: Vec<String>,
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
        // The last cell must have valid coordinates too
        for (offset, origin) in [(columns, stored.origin.0), (rows.len(), stored.origin.1)] {
            if offset > 0 && origin.checked_add(offset as i32 - 1).is_none() {
                return Err(D::Error::custom("tilemap cells out of range"));
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
        if !cells.is_empty() {
            tilemap.origin = stored.origin;
            tilemap.size = (columns as u32, rows.len() as u32);
            tilemap.cells = cells;
        }
        Ok(tilemap)
    }
}
