//! RNG and procedural generation.

use std::convert::TryInto;

use ahash::{AHashMap, AHashSet};
use cogs_gamedev::grids::{Direction4, ICoord};
use hecs::World;
use macroquad::prelude::{Color, BLUE};
use nalgebra::vector;
use noise::{Billow, Blend, NoiseFn, Seedable, SuperSimplex};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use rapier2d::{
    prelude::{ColliderBuilder, InteractionGroups, RigidBodyBuilder},
    utils::WSign,
};

use crate::{
    modes::overworld::{
        cs::{colored_box::ColoredBox, physics::HasRigidBody, shrine::Shrine},
        physics::collider_groups,
    },
    utils::draw::hexcolor,
};

use super::{cs::player::Player, physics::PhysicsWorld, WorldExt};

/// Tile size in the world.
pub const WORLD_SIZE: isize = 128;

/// Abstraction layer over components: tiles representing structures.
/// These are 1x1 meters, or 16x16 pixels.
#[derive(Debug, Clone, Copy)]
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

impl Tile {
    fn is_solid(&self) -> bool {
        match self {
            Tile::Ground | Tile::Floor | Tile::Path => false,
            Tile::Rock | Tile::Wall => true,
        }
    }

    fn color(&self) -> Color {
        match self {
            Tile::Ground => hexcolor(0x617464_ff),
            Tile::Rock => hexcolor(0x545246_ff),
            Tile::Wall => hexcolor(0x615e4c_ff),
            Tile::Floor => hexcolor(0x7e8c6d_ff),
            Tile::Path => hexcolor(0x6b786d_ff),
        }
    }
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
    let mut state = TileMap {
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

    state.generate();

    // for y in 0..WORLD_SIZE {
    //     let line = (0..WORLD_SIZE)
    //         .map(|x| {
    //             let pos = ICoord::new(x, y);
    //             let tile = state.tiles.get(&pos).unwrap();
    //             if tile.is_solid() {
    //                 '#'
    //             } else {
    //                 ' '
    //             }
    //         })
    //         .collect::<String>();
    //     println!("{}", &line);
    // }

    let map = state.tiles;
    let mut rng = state.rng;

    let mut open_spots = Vec::new();
    for (pos, tile) in map {
        // Their coordinate positions become their translations.
        // (that sounds poetic)
        if !tile.is_solid() {
            open_spots.push(pos);
        }

        let color = tile.color();
        let filter = if tile.is_solid() {
            collider_groups::FILTER_WALLS
        } else {
            0x0
        };

        let coll = ColliderBuilder::cuboid(0.5, 0.5)
            .collision_groups(InteractionGroups::new(collider_groups::GROUP_WALLS, filter))
            .build();
        let rb = RigidBodyBuilder::new_static()
            .translation(vector![pos.x as f32, pos.y as f32])
            .build();
        world.spawn_with_physics(physics, (ColoredBox(color),), coll, Some(rb));
    }

    let player_spot = rng.gen_range(0..open_spots.len());
    let shrine_spot = rng.gen_range(0..open_spots.len());
    let player_spot = open_spots[player_spot];
    let shrine_spot = open_spots[shrine_spot];

    // Move player
    {
        let player_h = world.get_player();
        let rb_h = world.get::<HasRigidBody>(player_h).unwrap();
        let rb = physics.rigid_bodies.get_mut(**rb_h).unwrap();
        // It's ok to teleport the player to somewhere empty
        rb.set_translation(vector![player_spot.x as f32, player_spot.y as f32], false);
    }
    let coll = ColliderBuilder::cuboid(0.4, 0.4)
        .translation(vector![shrine_spot.x as f32, shrine_spot.y as f32])
        .collision_groups(InteractionGroups::none())
        .build();
    world.spawn_with_physics(physics, (Shrine::new(1), ColoredBox(BLUE)), coll, None);
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
    fn generate(&mut self) {
        self.fill();
        self.carve_caves();
    }

    /// Fill everything with Rock
    fn fill(&mut self) {
        for x in 0..WORLD_SIZE {
            for y in 0..WORLD_SIZE {
                let pos = ICoord::new(x, y);
                let tile = Tile::Rock;
                self.tiles.insert(pos, tile);
            }
        }
    }

    fn carve_caves(&mut self) {
        // there are so many possibilities ...
        // for now we will use a Growing Tree algorithm, then make it less
        // jaggedy with a perlin noise.

        let mut billow = Billow::new().set_seed(self.rng.gen());
        billow.frequency = 5.0;
        billow.octaves = 2;
        let simplex = SuperSimplex::new().set_seed(self.rng.gen());
        let noiser = Blend::<'_, [f64; 2]>::new(&billow, &simplex, &simplex);

        // Positive values are stone; negative are ground
        let mut hardnesses = AHashMap::new();

        {
            let x = self.rng.gen_range(0..WORLD_SIZE);
            let y = self.rng.gen_range(0..WORLD_SIZE);
            let origin = ICoord::new(x, y);

            // If a pos in this set it ought to be empty
            let mut empties = AHashSet::new();
            empties.insert(origin);
            let mut exposed = Direction4::DIRECTIONS
                .iter()
                .map(|dir| origin + *dir)
                .collect::<Vec<_>>();

            let distr = rand_distr::Exp::new(0.5f32).unwrap();

            let in_bounds =
                |pos: ICoord| pos.x >= 0 && pos.x < WORLD_SIZE && pos.y >= 0 && pos.y <= WORLD_SIZE;

            while !exposed.is_empty() {
                let idx = self.rng.sample(distr);
                let idx = (exposed.len() - 1).saturating_sub(idx.round() as usize);
                let ex = exposed.remove(idx);

                let open_adjacent_count = Direction4::DIRECTIONS
                    .iter()
                    .filter(|dir| {
                        let pos = ex + **dir;
                        in_bounds(pos) && empties.contains(&pos)
                    })
                    .count();
                if open_adjacent_count == 1 {
                    // make this open!
                    empties.insert(ex);
                    for dir in Direction4::DIRECTIONS {
                        let pos = ex + dir;
                        if in_bounds(pos) && !empties.contains(&pos) {
                            exposed.push(pos);
                        }
                    }
                }
            }

            for x in 0..WORLD_SIZE {
                for y in 0..WORLD_SIZE {
                    let pos = ICoord::new(x, y);
                    let hardness = if empties.contains(&pos) { -0.7f32 } else { 0.4 };
                    hardnesses.insert(pos, hardness);
                }
            }
        }

        for (pos, hardness) in hardnesses.iter_mut() {
            let sampler = [pos.x as f64 / 40.0, pos.y as f64 / 40.0];
            let noise = noiser.get(sampler);
            *hardness += noise as f32;
        }

        // Finally ...
        for (pos, hardness) in hardnesses {
            let tile = if hardness > 0.2 {
                Tile::Rock
            } else {
                Tile::Ground
            };
            self.tiles.insert(pos, tile);
        }
    }
}
