mod cs;
mod physics;
mod procgen;
mod spells;

use crate::{
    assets::Assets,
    boilerplates::{FrameInfo, Gamemode, Transition},
    controls::InputSubscriber,
    modes::overworld::{
        cs::{
            debug::system_draw_collision,
            particles::system_draw_particles,
            physics::{system_run_physics, HasCollider, HasRigidBody},
            player::{player_body_collider, system_draw_spellcaster, system_player_inputs, Player},
            projectiles::system_draw_projectiles,
        },
        physics::PhysicsWorld,
        spells::{
            casting::PatternDrawState,
            patterns::{RawPattern, HEX_WIDTH},
        },
    },
    HEIGHT, WIDTH,
};

use hecs::{ComponentError, Entity, NoSuchEntity, World};
use macroquad::prelude::RenderTarget;
use nalgebra::{Matrix3, Similarity2, Vector2};
use rapier2d::prelude::*;

use self::cs::{
    dazing::{system_dazed, Dazeable},
    explosions::system_explosions,
    limited_time_offer::system_limited_timers,
    particles::system_cleanup_particles,
    projectiles::system_projectiles,
};

/// Mode for the main playing state with the player running around dungeons.
pub struct ModeOverworld {
    /// Big soup of entities.
    world: World,
    /// Physics engine stuff
    physics: PhysicsWorld,
}

impl ModeOverworld {
    pub fn init() -> Self {
        let mut world = World::new();
        let mut physics = PhysicsWorld::new();

        let map = r"
 # # # # #
      
 #       #
    ###
 #  # #  #

 # ##### #

 # # # # #
";
        for (y, line) in map.lines().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                // Spawn walls
                if ch == '#' {
                    let x = x as f32 - 6.0;
                    let y = y as f32 - 6.0;
                    world.spawn_with_physics(
                        &mut physics,
                        (),
                        // Cuboids are defined by *half*-extents, so we give it
                        // half the w and h
                        ColliderBuilder::cuboid(0.5, 0.5).build(),
                        Some(
                            RigidBodyBuilder::new_static()
                                .translation(vector![x, y])
                                .build(),
                        ),
                    );
                }
            }
        }

        let (coll, rb) = player_body_collider();
        world.spawn_with_physics(
            &mut physics,
            (Player::new(), Dazeable::new()),
            coll,
            Some(rb),
        );

        ModeOverworld { world, physics }
    }
}

impl Gamemode for ModeOverworld {
    fn update(
        &mut self,
        controls: &InputSubscriber,
        frame_info: FrameInfo,
        assets: &Assets,
    ) -> Transition {
        system_explosions(&mut self.world, &mut self.physics);

        system_player_inputs(&mut self.world, &mut self.physics, controls);
        system_dazed(&mut self.world, &mut self.physics);

        system_run_physics(&mut self.world, &mut self.physics);

        system_cleanup_particles(&mut self.world, &mut self.physics);
        system_projectiles(&mut self.world, &mut self.physics);
        system_limited_timers(&mut self.world, &mut self.physics);

        Transition::None
    }

    fn draw(&self, assets: &Assets, frame_info: FrameInfo, controls: &InputSubscriber) {
        use macroquad::prelude::*;

        clear_background(BLACK);

        let player_id = self.world.get_player();
        let handle = self.world.get::<HasCollider>(player_id).unwrap().0;
        let collider = self.physics.colliders.get(handle).unwrap();
        // `pos` is the center of the shape. how convenient.
        let camera_pos = collider.compute_aabb().center();

        let canvas = render_target(WIDTH as u32, HEIGHT as u32);
        canvas.texture.set_filter(FilterMode::Nearest);

        push_camera_state();
        set_camera(&Camera2D {
            render_target: Some(canvas),
            target: (camera_pos).into(),
            zoom: vec2(2.0 / WIDTH, 2.0 / HEIGHT) * 16.0,
            ..Default::default()
        });

        system_draw_projectiles(&self.world, &self.physics);
        system_draw_particles(&self.world, &self.physics);

        // just do some debug drawing for now
        system_draw_collision(&self.world, &self.physics);

        pop_camera_state();
        draw_texture(canvas.texture, 0.0, 0.0, WHITE);
        system_draw_spellcaster(&self.world, controls);
    }
}

trait WorldExt {
    /// Get the player's ID (specifically, the first Entity with a Player component).
    ///
    /// Panics if it can't find the player.
    fn get_player(&self) -> Entity;

    /// Add an entity with the given components to the world,
    /// and physics information to the physics world.
    ///
    /// If `body` is None, only the Collider will be added.
    /// Otherwise it will be added and the collider will be attached to it.
    ///
    /// This adds the `HasCollider` and `HasRigidBody` components for you (if applicable), and
    /// adds the entity as userdata on them.
    fn spawn_with_physics(
        &mut self,
        physics: &mut PhysicsWorld,
        components: impl hecs::DynamicBundle,
        collider: Collider,
        body: Option<RigidBody>,
    ) -> Entity;

    /// Despawn the entity, and if it can removes colliders and bodies from it.
    fn despawn_with_physics(
        &mut self,
        physics: &mut PhysicsWorld,
        entity: Entity,
    ) -> Result<(), ComponentError>;
}

impl WorldExt for World {
    fn get_player(&self) -> Entity {
        let mut query = self.query::<&Player>();
        if let Some((player, _)) = query.iter().next() {
            player
        } else {
            panic!("could not find any entity with player component")
        }
    }

    fn spawn_with_physics(
        &mut self,
        physics: &mut PhysicsWorld,
        components: impl hecs::DynamicBundle,
        mut collider: Collider,
        body: Option<RigidBody>,
    ) -> Entity {
        let e = self.spawn(components);
        collider.user_data = e.to_bits() as u128;

        if let Some(mut body) = body {
            body.user_data = e.to_bits() as u128;

            let body_handle = physics.rigid_bodies.insert(body);
            let collider_handle = physics.colliders.insert_with_parent(
                collider,
                body_handle,
                &mut physics.rigid_bodies,
            );
            self.insert(e, (HasRigidBody(body_handle), HasCollider(collider_handle)))
                .unwrap();
        } else {
            let collider_handle = physics.colliders.insert(collider);
            self.insert_one(e, HasCollider(collider_handle)).unwrap();
        };

        e
    }

    /// Despawn the entity, and if it can removes colliders and bodies from it.
    fn despawn_with_physics(
        &mut self,
        physics: &mut PhysicsWorld,
        entity: Entity,
    ) -> Result<(), ComponentError> {
        if let Ok(rb_handle) = self.get::<HasRigidBody>(entity) {
            // the remove method conveniently removes colliders and joints and stuff
            physics.rigid_bodies.remove(
                rb_handle.0,
                &mut physics.island_manager,
                &mut physics.colliders,
                &mut physics.joints,
            );
        }
        if let Ok(coll_handle) = self.get::<HasCollider>(entity) {
            // in case this is just a collider and not a rb attached
            // no need to wake it up, if the rigid body existed it was just removed
            physics.colliders.remove(
                coll_handle.0,
                &mut physics.island_manager,
                &mut physics.rigid_bodies,
                false,
            );
        }
        self.despawn(entity)?;
        Ok(())
    }
}
