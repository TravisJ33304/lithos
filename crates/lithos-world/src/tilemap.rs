//! Tilemap system for the Lithos overworld.
//!
//! The overworld is divided into 32×32-tile chunks, where each tile is 40 world units.
//! This gives a chunk size of 1280×1280 world units. Terrain, ceiling type, and height
//! are all stored per-tile and generated deterministically from the world seed.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use noise::{NoiseFn, Perlin};

/// Width/height of a chunk in tiles.
pub const CHUNK_SIZE: usize = 32;

/// Size of one tile in world units.
pub const TILE_SIZE: f32 = 40.0;

/// Size of a chunk in world units.
pub const CHUNK_WORLD_SIZE: f32 = CHUNK_SIZE as f32 * TILE_SIZE; // 1280.0

/// Convert a world position to tile coordinates.
pub fn world_to_tile(pos: lithos_protocol::Vec2) -> (i32, i32) {
    (
        (pos.x / TILE_SIZE).floor() as i32,
        (pos.y / TILE_SIZE).floor() as i32,
    )
}

/// Convert tile coordinates to the center world position of that tile.
pub fn tile_to_world(tile: (i32, i32)) -> lithos_protocol::Vec2 {
    lithos_protocol::Vec2::new(
        tile.0 as f32 * TILE_SIZE + TILE_SIZE * 0.5,
        tile.1 as f32 * TILE_SIZE + TILE_SIZE * 0.5,
    )
}

/// Coordinate of a chunk in chunk-space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    /// Convert a world position to the chunk that contains it.
    pub fn from_world_pos(pos: lithos_protocol::Vec2) -> Self {
        Self {
            x: (pos.x / CHUNK_WORLD_SIZE).floor() as i32,
            y: (pos.y / CHUNK_WORLD_SIZE).floor() as i32,
        }
    }

    /// Get the world-space center of this chunk.
    pub fn world_center(&self) -> lithos_protocol::Vec2 {
        lithos_protocol::Vec2::new(
            (self.x as f32 + 0.5) * CHUNK_WORLD_SIZE,
            (self.y as f32 + 0.5) * CHUNK_WORLD_SIZE,
        )
    }
}

/// The type of terrain on a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainType {
    /// Open space — passable by ground and flying units.
    Empty,
    /// Solid rock outcropping — impassable to ground units and projectiles.
    Rock,
    /// Deep chasm or crater edge — impassable to ground units.
    DeepRavine,
    /// Dense asteroid debris field — passable but may slow movement.
    AsteroidField,
    /// Automata structure spire — impassable, acts as a spawn point.
    AutomataSpire,
}

/// Whether a tile has an open sky or an enclosed ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CeilingType {
    /// Open to space — flying units can pass.
    Open,
    /// Pressurized / enclosed — flying units (Drones) are blocked.
    Enclosed,
}

/// A single tile in the overworld tilemap.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub terrain: TerrainType,
    pub ceiling: CeilingType,
    /// Visual height variation (0–255). Higher = more elevation.
    pub height: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain: TerrainType::Empty,
            ceiling: CeilingType::Open,
            height: 0,
        }
    }
}

impl Tile {
    /// Is this tile passable for ground-based movement?
    pub fn is_ground_passable(&self) -> bool {
        matches!(
            self.terrain,
            TerrainType::Empty | TerrainType::AsteroidField
        )
    }

    /// Is this tile passable for flying movement?
    pub fn is_flying_passable(&self) -> bool {
        self.ceiling == CeilingType::Open
    }

    /// Is this tile passable for projectiles?
    pub fn is_projectile_passable(&self) -> bool {
        // Projectiles are blocked by solid terrain and enclosed ceilings.
        self.is_ground_passable() && self.ceiling == CeilingType::Open
    }
}

/// A 32×32 chunk of tiles.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub tiles: Vec<Tile>,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            tiles: vec![Tile::default(); CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    /// Get a tile by its in-chunk coordinates (0..CHUNK_SIZE).
    pub fn get(&self, local_x: usize, local_y: usize) -> Option<&Tile> {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return None;
        }
        self.tiles.get(local_y * CHUNK_SIZE + local_x)
    }

    /// Get a mutable tile by its in-chunk coordinates.
    pub fn get_mut(&mut self, local_x: usize, local_y: usize) -> Option<&mut Tile> {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return None;
        }
        self.tiles.get_mut(local_y * CHUNK_SIZE + local_x)
    }

    /// Convert a world position to local chunk tile coordinates.
    pub fn world_to_local(pos: lithos_protocol::Vec2, coord: ChunkCoord) -> (usize, usize) {
        let local_x = ((pos.x - coord.x as f32 * CHUNK_WORLD_SIZE) / TILE_SIZE)
            .floor()
            .clamp(0.0, (CHUNK_SIZE - 1) as f32) as usize;
        let local_y = ((pos.y - coord.y as f32 * CHUNK_WORLD_SIZE) / TILE_SIZE)
            .floor()
            .clamp(0.0, (CHUNK_SIZE - 1) as f32) as usize;
        (local_x, local_y)
    }
}

/// Global tilemap resource stored in the ECS World.
#[derive(Resource, Debug)]
pub struct TileMap {
    pub chunks: HashMap<ChunkCoord, Chunk>,
    pub seed: u32,
}

impl TileMap {
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            seed,
        }
    }

    /// Ensure a chunk exists, generating it on demand if necessary.
    pub fn ensure_chunk(&mut self, coord: ChunkCoord) -> &Chunk {
        if !self.chunks.contains_key(&coord) {
            let chunk = generate_chunk(coord, self.seed);
            self.chunks.insert(coord, chunk);
        }
        self.chunks.get(&coord).unwrap()
    }

    /// Get a chunk if it already exists (does not generate).
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    /// Get a tile at a world position, generating the chunk if needed.
    pub fn get_tile(&mut self, pos: lithos_protocol::Vec2) -> Option<&Tile> {
        let coord = ChunkCoord::from_world_pos(pos);
        self.ensure_chunk(coord);
        let chunk = self.chunks.get(&coord)?;
        let (lx, ly) = Chunk::world_to_local(pos, coord);
        chunk.get(lx, ly)
    }

    /// Get a tile at a world position without generating (returns None if chunk not loaded).
    pub fn get_tile_loaded(&self, pos: lithos_protocol::Vec2) -> Option<&Tile> {
        let coord = ChunkCoord::from_world_pos(pos);
        let chunk = self.chunks.get(&coord)?;
        let (lx, ly) = Chunk::world_to_local(pos, coord);
        chunk.get(lx, ly)
    }

    /// Check if a world position is passable for a given movement type.
    pub fn is_passable(&mut self, pos: lithos_protocol::Vec2, flying: bool) -> bool {
        match self.get_tile(pos) {
            Some(tile) => {
                if flying {
                    tile.is_flying_passable()
                } else {
                    tile.is_ground_passable()
                }
            }
            None => false,
        }
    }

    /// Check passability without generating chunks.
    pub fn is_passable_loaded(&self, pos: lithos_protocol::Vec2, flying: bool) -> bool {
        match self.get_tile_loaded(pos) {
            Some(tile) => {
                if flying {
                    tile.is_flying_passable()
                } else {
                    tile.is_ground_passable()
                }
            }
            None => false,
        }
    }

    /// Collect all chunk coordinates within a given radius (in chunks) of a center chunk.
    pub fn chunks_in_radius(center: ChunkCoord, radius: i32) -> Vec<ChunkCoord> {
        let mut coords = Vec::with_capacity(((radius * 2 + 1).pow(2)) as usize);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                coords.push(ChunkCoord {
                    x: center.x + dx,
                    y: center.y + dy,
                });
            }
        }
        coords
    }

    /// Unload chunks that are farther than `radius` chunks from any active loader position.
    pub fn unload_distant_chunks(
        &mut self,
        loader_positions: &[lithos_protocol::Vec2],
        radius: i32,
    ) {
        let mut keep = std::collections::HashSet::new();
        for pos in loader_positions {
            let center = ChunkCoord::from_world_pos(*pos);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    keep.insert(ChunkCoord {
                        x: center.x + dx,
                        y: center.y + dy,
                    });
                }
            }
        }
        self.chunks.retain(|coord, _| keep.contains(coord));
    }

    /// Run A* pathfinding from `start` to `goal` for ground units.
    /// Returns a list of world positions representing the path, or None if no path exists.
    /// `max_distance` limits the search radius in tiles to prevent excessive computation.
    pub fn find_path_ground(
        &mut self,
        start: lithos_protocol::Vec2,
        goal: lithos_protocol::Vec2,
        max_distance: usize,
    ) -> Option<Vec<lithos_protocol::Vec2>> {
        use std::collections::{BinaryHeap, HashSet};

        let start_coord = world_to_tile(start);
        let goal_coord = world_to_tile(goal);

        // Early exit if start or goal is impassable.
        if !self.is_passable(start, false) || !self.is_passable(goal, false) {
            return None;
        }

        let heuristic = |a: (i32, i32), b: (i32, i32)| -> f32 {
            let dx = (a.0 - b.0) as f32;
            let dy = (a.1 - b.1) as f32;
            (dx * dx + dy * dy).sqrt()
        };

        #[derive(Debug, Clone, Copy, PartialEq)]
        struct Node {
            cost: f32,
            pos: (i32, i32),
        }
        impl Eq for Node {}
        impl Ord for Node {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                other.cost.partial_cmp(&self.cost).unwrap()
            }
        }
        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut open = BinaryHeap::new();
        let mut closed = HashSet::new();
        let mut came_from = HashMap::new();
        let mut g_score = HashMap::new();

        open.push(Node {
            cost: heuristic(start_coord, goal_coord),
            pos: start_coord,
        });
        g_score.insert(start_coord, 0.0f32);

        let directions = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        while let Some(current) = open.pop() {
            if current.pos == goal_coord {
                // Reconstruct path.
                let mut path = Vec::new();
                let mut cur = goal_coord;
                while cur != start_coord {
                    path.push(tile_to_world(cur));
                    cur = *came_from.get(&cur)?;
                }
                path.reverse();
                return Some(path);
            }

            if closed.contains(&current.pos) {
                continue;
            }
            closed.insert(current.pos);

            if closed.len() > max_distance * max_distance {
                return None; // Search limit exceeded.
            }

            let current_g = *g_score.get(&current.pos).unwrap_or(&f32::MAX);

            for &(dx, dy) in &directions {
                let neighbor = (current.pos.0 + dx, current.pos.1 + dy);
                let neighbor_world = tile_to_world(neighbor);

                if !self.is_passable(neighbor_world, false) {
                    continue;
                }

                let move_cost = if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
                let tentative_g = current_g + move_cost;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&f32::MAX) {
                    came_from.insert(neighbor, current.pos);
                    g_score.insert(neighbor, tentative_g);
                    let f = tentative_g + heuristic(neighbor, goal_coord);
                    open.push(Node {
                        cost: f,
                        pos: neighbor,
                    });
                }
            }
        }

        None
    }
}

impl Default for TileMap {
    fn default() -> Self {
        Self::new(12345)
    }
}

// ---------------------------------------------------------------------------
// Procedural chunk generation
// ---------------------------------------------------------------------------

/// Deterministically generate a single chunk from its coordinate and world seed.
fn generate_chunk(coord: ChunkCoord, world_seed: u32) -> Chunk {
    let mut chunk = Chunk::new(coord);

    // Derive a per-chunk seed so each chunk is deterministic but unique.
    let chunk_seed = world_seed
        .wrapping_add(coord.x as u32)
        .wrapping_mul(374761393)
        .wrapping_add(coord.y as u32)
        .wrapping_mul(668265263);

    let perlin = Perlin::new(chunk_seed);
    let perlin2 = Perlin::new(chunk_seed.wrapping_add(1));
    let perlin3 = Perlin::new(chunk_seed.wrapping_add(2));

    for ly in 0..CHUNK_SIZE {
        for lx in 0..CHUNK_SIZE {
            let world_x = coord.x as f32 * CHUNK_WORLD_SIZE + lx as f32 * TILE_SIZE;
            let world_y = coord.y as f32 * CHUNK_WORLD_SIZE + ly as f32 * TILE_SIZE;
            let pos = lithos_protocol::Vec2::new(world_x, world_y);

            let tile = generate_tile(pos, &perlin, &perlin2, &perlin3);
            chunk.tiles[ly * CHUNK_SIZE + lx] = tile;
        }
    }

    chunk
}

/// Generate a single tile at a world position using multiple noise octaves.
fn generate_tile(
    pos: lithos_protocol::Vec2,
    terrain_noise: &Perlin,
    ravine_noise: &Perlin,
    height_noise: &Perlin,
) -> Tile {
    let dist = pos.length();
    let dist_normalized = (dist / 4000.0).clamp(0.0, 1.0);

    // Primary terrain noise (smooth over large areas).
    let terrain_val = terrain_noise.get([pos.x as f64 / 800.0, pos.y as f64 / 800.0]);

    // Ravine noise (directional ridges).
    let ravine_val = ravine_noise.get([pos.x as f64 / 400.0, pos.y as f64 / 1200.0]);

    // Height noise (visual variation).
    let height_val = height_noise.get([pos.x as f64 / 200.0, pos.y as f64 / 200.0]);
    let height = ((height_val + 1.0) * 127.5) as u8;

    // Radial difficulty bias:
    // - Outer Rim (dist > ~2500): mostly empty, few rocks
    // - Mid-Zone (dist ~1000–2500): mixed rocks, ravines, asteroid fields
    // - Core (dist < ~1000): dense rocks, spires, ravines

    let rock_threshold = (-0.2 + dist_normalized * 0.6) as f64; // easier rocks near center
    let ravine_threshold = (0.75 - dist_normalized * 0.3) as f64; // more ravines near center

    let terrain = if terrain_val > rock_threshold {
        // Rock formation
        if dist < 800.0 && terrain_val > 0.6 {
            // Core spires
            TerrainType::AutomataSpire
        } else if ravine_val > ravine_threshold {
            TerrainType::DeepRavine
        } else {
            TerrainType::Rock
        }
    } else if terrain_val > rock_threshold - 0.15 {
        // Transition zone = asteroid field
        TerrainType::AsteroidField
    } else {
        TerrainType::Empty
    };

    // Ceiling: everything is Open by default. Enclosed is only set for
    // specific structures (POIs, bases) after generation.
    let ceiling = CeilingType::Open;

    Tile {
        terrain,
        ceiling,
        height,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_coord_from_world_pos() {
        // Position at origin → chunk (0, 0)
        assert_eq!(
            ChunkCoord::from_world_pos(lithos_protocol::Vec2::new(0.0, 0.0)),
            ChunkCoord { x: 0, y: 0 }
        );

        // Position at chunk boundary → next chunk
        assert_eq!(
            ChunkCoord::from_world_pos(lithos_protocol::Vec2::new(1280.0, 0.0)),
            ChunkCoord { x: 1, y: 0 }
        );

        // Negative position
        assert_eq!(
            ChunkCoord::from_world_pos(lithos_protocol::Vec2::new(-1.0, -1.0)),
            ChunkCoord { x: -1, y: -1 }
        );
    }

    #[test]
    fn test_deterministic_generation() {
        let seed = 42;
        let coord = ChunkCoord { x: 0, y: 0 };
        let chunk1 = generate_chunk(coord, seed);
        let chunk2 = generate_chunk(coord, seed);

        assert_eq!(chunk1.tiles.len(), chunk2.tiles.len());
        for i in 0..chunk1.tiles.len() {
            assert_eq!(chunk1.tiles[i].terrain as u8, chunk2.tiles[i].terrain as u8);
            assert_eq!(chunk1.tiles[i].ceiling as u8, chunk2.tiles[i].ceiling as u8);
        }
    }

    #[test]
    fn test_tile_passability() {
        let empty = Tile {
            terrain: TerrainType::Empty,
            ceiling: CeilingType::Open,
            height: 0,
        };
        let rock = Tile {
            terrain: TerrainType::Rock,
            ceiling: CeilingType::Open,
            height: 0,
        };
        let enclosed = Tile {
            terrain: TerrainType::Empty,
            ceiling: CeilingType::Enclosed,
            height: 0,
        };

        assert!(empty.is_ground_passable());
        assert!(empty.is_flying_passable());

        assert!(!rock.is_ground_passable());
        assert!(rock.is_flying_passable()); // open ceiling

        assert!(enclosed.is_ground_passable());
        assert!(!enclosed.is_flying_passable());
    }
}
