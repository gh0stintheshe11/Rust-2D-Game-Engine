//! Tilemaps: the grid of tiles, and how scene files store it.

use rust_2d_game_engine::scene::{Entity, MAX_TILEMAP_SIDE, TileCollision, Tilemap};
use std::path::PathBuf;

fn tilemap() -> Tilemap {
    Tilemap::new(PathBuf::from("/game/assets/images/tiles.png"), (16, 16))
}

#[test]
fn test_cells_grow_in_every_direction() {
    let mut map = tilemap();
    assert_eq!(map.get(0, 0), None);
    assert_eq!(map.bounds(), None);
    assert_eq!(map.set(0, 0, None), Ok(false), "emptying an empty cell");

    assert_eq!(map.set(2, 1, Some(5)), Ok(true));
    assert_eq!(map.set(-3, -2, Some(0)), Ok(true));
    assert_eq!(map.set(2, 1, Some(5)), Ok(false), "unchanged");
    assert_eq!(map.get(2, 1), Some(5));
    assert_eq!(map.get(-3, -2), Some(0));
    assert_eq!(map.get(0, 0), None, "between the tiles");
    assert_eq!(map.tile_count(), 2);
    assert_eq!(map.bounds(), Some(((-3, -2), (6, 4))));
    assert_eq!(
        map.tiles().collect::<Vec<_>>(),
        vec![((-3, -2), 0), ((2, 1), 5)],
        "row by row"
    );
    assert_eq!(
        map.tiles_in((0, 10), (-10, 10)).collect::<Vec<_>>(),
        vec![((2, 1), 5)],
        "only the asked columns"
    );

    assert_eq!(map.set(-3, -2, None), Ok(true));
    assert_eq!(map.bounds(), Some(((2, 1), (1, 1))));
    map.clear();
    assert_eq!(map.tile_count(), 0);
}

#[test]
fn test_maps_have_a_size_limit() {
    let mut map = tilemap();
    map.set(0, 0, Some(1)).unwrap();
    let far = MAX_TILEMAP_SIDE as i32;
    assert!(map.set(far, 0, Some(1)).is_err(), "too wide");
    assert!(map.set(0, -far, Some(1)).is_err(), "too tall");
    assert!(map.set(far - 1, 0, Some(1)).is_ok(), "exactly the limit");
    assert!(
        map.set(0, 0, Some(u32::MAX)).is_err(),
        "tile number overflow"
    );
}

#[test]
fn test_world_positions_to_cells() {
    let map = tilemap();
    let origin = (100.0, 50.0);
    assert_eq!(map.cell_at(origin, (100.0, 50.0)), (0, 0));
    assert_eq!(map.cell_at(origin, (131.9, 65.9)), (1, 0));
    assert_eq!(
        map.cell_at(origin, (99.0, 49.0)),
        (-1, -1),
        "left of and above"
    );
    assert_eq!(map.cell_at(origin, (f32::NAN, 1e30)), (0, i32::MAX));
}

#[test]
fn test_scene_files_store_the_area_with_tiles() {
    let mut map = tilemap();
    map.set(10, 5, Some(3)).unwrap();
    map.set(12, 6, Some(0)).unwrap();
    // Erased edges don't count
    map.set(40, 40, Some(1)).unwrap();
    map.set(40, 40, None).unwrap();

    let json = serde_json::to_value(&map).unwrap();
    assert_eq!(json["origin"], serde_json::json!([10, 5]));
    assert_eq!(json["rows"], serde_json::json!(["3 . .", ". . 0"]));
    let read: Tilemap = serde_json::from_value(json).unwrap();
    assert_eq!(read.get(10, 5), Some(3));
    assert_eq!(read.get(12, 6), Some(0));
    assert_eq!(read.tile_count(), 2);
    assert_eq!(read.bounds(), map.bounds());

    for bad in [
        serde_json::json!({"tileset": "t.png", "tile_size": [16, 16], "origin": [0, 0], "rows": ["1 x"]}),
        serde_json::json!({"tileset": "t.png", "tile_size": [16, 16], "origin": [i32::MAX, 0], "rows": ["1 1"]}),
        serde_json::json!({"tileset": "t.png", "tile_size": [16, 16], "origin": [0, 0], "rows": ["4294967295"]}),
    ] {
        assert!(
            serde_json::from_value::<Tilemap>(bad.clone()).is_err(),
            "{}",
            bad
        );
    }
}

#[test]
fn test_entities_without_a_tilemap_store_none() {
    let entity = Entity::new(uuid::Uuid::new_v4(), "plain").unwrap();
    let json = serde_json::to_value(&entity).unwrap();
    assert!(
        json.get("tilemap").is_none(),
        "old scene files stay as they were"
    );
    let read: Entity = serde_json::from_value(json).unwrap();
    assert!(read.tilemap.is_none());
}

#[test]
fn test_tiles_collide_as_set_per_tile() {
    let mut map = tilemap();
    map.set(0, 0, Some(3)).unwrap();
    assert_eq!(map.tile_collision(3), TileCollision::Solid, "by default");
    map.set_tile_collision(3, TileCollision::OneWay);
    map.set_tile_collision(5, TileCollision::Passable);
    assert_eq!(map.tile_collision(3), TileCollision::OneWay);

    // Scene files list the tiles that aren't solid
    let json = serde_json::to_value(&map).unwrap();
    assert_eq!(
        json["collision"],
        serde_json::json!({"3": "one_way", "5": "passable"})
    );
    let read: Tilemap = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(read, map);

    // Solid again: not listed, and a map without the key is all solid
    map.set_tile_collision(3, TileCollision::Solid);
    map.set_tile_collision(5, TileCollision::Solid);
    let json = serde_json::to_value(&map).unwrap();
    assert!(json.get("collision").is_none(), "{}", json);
    assert_eq!(serde_json::from_value::<Tilemap>(json).unwrap(), map);

    let mut bad = serde_json::to_value(&map).unwrap();
    bad["collision"] = serde_json::json!({"1": "bouncy"});
    assert!(serde_json::from_value::<Tilemap>(bad).is_err());
}
