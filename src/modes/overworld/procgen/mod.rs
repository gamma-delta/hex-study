//! RNG and procedural generation.

use std::convert::TryInto;

use ahash::AHashMap;
use cogs_gamedev::grids::{Direction4, ICoord};
use hecs::World;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use rapier2d::utils::WSign;

use super::{cs::player::Player, physics::PhysicsWorld, WorldExt};

/// Tile size in the world.
pub const WORLD_SIZE: isize = 128;

/// Abstraction layer over components: tiles representing structures.
/// These are 1x1 meters, or 16x16 pixels.
enum Tile {
    /// Normal ground
    Ground,
    /// Solid blocks that weren't created by people
    Rock,
    /// Solid blocks that *were* created by people
    Wall,
    /// Tiles inside houses and buildings
    Floor,
    /// Things connecting buildings
    Path,
}

/// Remove all entities except for the player, generate a map at the given depth,
/// and add the new things to the world.
pub fn generate_map(seed: u64, depth: u64, world: &mut World, physics: &mut PhysicsWorld) {
    // From the rand docs:
    // PRNGs: Several companion crates are available,
    // providing individual or families of PRNG algorithms.
    // These provide the implementations behind StdRng and SmallRng but can also be used directly,
    // indeed should be used directly when reproducibility matters.
    // Some suggestions are: rand_chacha, rand_pcg, rand_xoshiro.
    // A full list can be found by searching for crates with the rng tag.

    // With this in mind I am using rand_xoshiro, mostly because
    // - it's very fast
    // - it's written by the fastutil people and i figure i owe them to use
    //   a library of theirs while not sobbing because i have to use java

    // Xoshiro wants 32 u8s, but i only have 16 in the input.
    // so i do a little mixing.
    // Hope this is OK
    let seed_split = [
        seed.to_le_bytes(),
        depth.to_le_bytes(),
        (!depth).to_be_bytes(),
        (!seed).to_be_bytes(),
    ];
    let rng_seed = seed_split.concat();
    let rng = Xoshiro256StarStar::from_seed(rng_seed.try_into().unwrap());
    let state = TileMap {
        tiles: AHashMap::new(),
        rng,
    };

    // Remove everything but the player
    {
        let mut remove = Vec::new();
        for (e, player) in world.query_mut::<Option<&Player>>() {
            if player.is_none() {
                remove.push(e);
            }
        }
        for e in remove {
            world.despawn_with_physics(physics, e).unwrap();
        }
    }

    let map = state.generate();
    for (pos, tile) in map {}

    todo!()
}

/// Tilemap generator using a persistent and generic rng.
///
/// The generator has 3 stages:
/// - Carve caves
/// - Place building seeds
/// - Iterate buildings
struct TileMap<R: Rng> {
    tiles: AHashMap<ICoord, Tile>,
    rng: R,
}

impl<R: Rng> TileMap<R> {
    fn generate(mut self) -> AHashMap<ICoord, Tile> {
        self.fill();
        self.carve_caves();
        self.tiles
    }

    /// Fill everything with Rock
    fn fill(&mut self) {
        for x in 0..WORLD_SIZE {
            for y in 0..WORLD_SIZE {
                if self.rng.gen_bool(0.8) {
                    let pos = ICoord::new(x, y);
                    self.tiles.insert(pos, Tile::Rock);
                }
            }
        }
    }

    fn carve_caves(&mut self) {
        // there are so many possibilities ...
        // for now we will use a Growing Tree algorithm.

        let x = self.rng.gen_range(0..WORLD_SIZE);
        let y = self.rng.gen_range(0..WORLD_SIZE);

        let mut exposed = vec![ICoord::new(x, y)];

        let distr = rand_distr::Exp::new(0.5f32).unwrap();
        while !exposed.is_empty() {
            let idx = self.rng.sample(distr);
            let idx = (exposed.len() - 1).saturating_sub(idx.round() as usize);

            let ex = exposed.remove(idx);
            let empty_adj = Direction4::DIRECTIONS
                .iter()
                .filter(|dir| {
                    let adj = self.tiles.get(&(ex + **dir));
                    matches!(adj, Some(Tile::Ground))
                })
                .count();
            if empty_adj <= 1 {
                for dir in Direction4::DIRECTIONS {
                    let preset = self.tiles.get(&(ex + dir));
                    if matches!(preset, Some(Tile::Ground)) {
                        exposed.push(ex + dir);
                    }
                }
            }
        }
    }
}
