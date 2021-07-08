mod cs;
pub mod damage;
mod physics;
mod procgen;
mod spells;

/// How much the player's velocity factors into the camera pos
const PLAYER_VEL_CAMERA_INFLUENCE: f32 = 2.2;
/// How much of the way to the target pos the camera pos tries to go
/// (factoring in the dt)
const CAMERA_SNAPPINESS: f32 = 0.8;
/// How far away the camera's position must be away from the target before
/// we move it
const CAMERA_TOLERANCE: f32 = 1.5;
/// If the camera is *too* far away, bring it back into this distance.
const CAMERA_MAX_DIST: f32 = 16.0;

/// How many light pixels there are across/down to one physics unit
const LIGHT_RESOLUTION: f32 = 1.0;

use crate::{
    assets::Assets,
    boilerplates::{FrameInfo, Gamemode, Transition},
    controls::{Control, InputSubscriber},
    modes::overworld::{
        cs::{
            colored_box::{system_draw_colored_boxes, ColoredBox},
            dazing::{system_dazed, Dazeable},
            debug::system_draw_collision,
            explosions::system_cleanup_explosions,
            light::{Illuminator, LightFalloffKind},
            limited_time_offer::system_cleanup_limited_timers,
            particles::system_cleanup_particles,
            particles::system_draw_particles,
            physics::{system_run_physics, HasCollider, HasRigidBody},
            player::{player_body_collider, system_draw_spellcaster, system_player_inputs, Player},
            projectiles::system_draw_projectiles,
            projectiles::system_update_and_cleanup_projectiles,
        },
        physics::{collider_groups, PhysicsWorld},
    },
    HEIGHT, WIDTH,
};

use cogs_gamedev::controls::InputHandler;
use hecs::{ComponentError, Entity, NoSuchEntity, World};
use macroquad::prelude::{
    info, vec3, Color, FilterMode, Image, Texture2D, Vec2, BLACK, BLANK, GRAY, ORANGE, WHITE,
};
use nalgebra::Point2;
use quad_rand::compat::QuadRand;
use rand::Rng;
use rapier2d::prelude::*;

use self::cs::damage::system_cleanup_dead;

/// Mode for the main playing state with the player running around dungeons.
pub struct ModeOverworld {
    /// Big soup of entities.
    world: World,
    /// Physics engine stuff
    physics: PhysicsWorld,

    /// Place where the camera is
    camera_pos: Vec2,
    /// Place where the camera targets
    camera_target: Vec2,

    /// Cached light data
    light_tex: Texture2D,
}

impl ModeOverworld {
    pub fn init() -> Self {
        let mut world = World::new();
        let mut physics = PhysicsWorld::new();

        let (coll, rb) = player_body_collider();
        world.spawn_with_physics(
            &mut physics,
            (
                Player::new(),
                Dazeable::new(),
                ColoredBox(ORANGE),
                Illuminator::new(vec3(1.0, 1.0, 0.9), LightFalloffKind::Circular { m: 0.1 }),
            ),
            coll,
            Some(rb),
        );

        let seed: u64 = QuadRand.gen();
        println!("seed: {}", seed);
        procgen::generate_map(seed, 0, &mut world, &mut physics);

        // new scope to appease borrowck
        let center = {
            let player = world.get_player().unwrap();
            let coll_h = world.get::<HasCollider>(player).unwrap();
            let coll = physics.colliders.get(**coll_h).unwrap();
            coll.compute_aabb().center().into()
        };

        // Make a dummy image to get the sizing right
        let lightmap = Image::gen_image_color(
            (WIDTH / 16.0 * LIGHT_RESOLUTION) as u16,
            (HEIGHT / 16.0 * LIGHT_RESOLUTION) as u16,
            BLACK,
        );
        let light_tex = Texture2D::from_image(&lightmap);
        light_tex.set_filter(FilterMode::Linear);

        ModeOverworld {
            world,
            physics,
            camera_pos: center,
            camera_target: center,
            light_tex,
        }
    }
}

impl Gamemode for ModeOverworld {
    fn update(
        &mut self,
        controls: &InputSubscriber,
        frame_info: FrameInfo,
        assets: &Assets,
    ) -> Transition {
        system_player_inputs(&mut self.world, &mut self.physics, controls);
        system_dazed(&mut self.world, &mut self.physics);

        system_run_physics(&mut self.world, &mut self.physics);

        system_update_and_cleanup_projectiles(&mut self.world, &mut self.physics);
        system_cleanup_limited_timers(&mut self.world, &mut self.physics);
        system_cleanup_particles(&mut self.world, &mut self.physics);
        system_cleanup_explosions(&mut self.world, &mut self.physics);
        system_cleanup_dead(&mut self.world, &mut self.physics);

        // To move the camera, we want
        if let Some(player_h) = self.world.get_player() {
            let (coll_h, rb_h) = self
                .world
                .query_one_mut::<(&HasCollider, &HasRigidBody)>(player_h)
                .unwrap();
            let coll = self.physics.colliders.get(**coll_h).unwrap();
            let rb = self.physics.rigid_bodies.get(**rb_h).unwrap();

            let pos = coll.compute_aabb().center();
            let vel = rb.linvel();

            self.camera_target = (pos + vel * PLAYER_VEL_CAMERA_INFLUENCE).into();
        }

        let cam_delta = self.camera_target - self.camera_pos;
        if cam_delta.length_squared() > CAMERA_TOLERANCE * CAMERA_TOLERANCE {
            let scaled = cam_delta * CAMERA_SNAPPINESS * self.physics.integration_params.dt;
            self.camera_pos += scaled;
        }

        Transition::None
    }

    fn draw(&self, assets: &Assets, frame_info: FrameInfo, controls: &InputSubscriber) {
        use macroquad::prelude::*;

        clear_background(BLACK);

        let canvas = render_target(WIDTH as u32, HEIGHT as u32);
        canvas.texture.set_filter(FilterMode::Nearest);

        // Round the camera pos to the nearest 1/16 to prevent driftiness
        let cam_x = (self.camera_pos.x * 16.0).round() / 16.0;
        let cam_y = (self.camera_pos.y * 16.0).round() / 16.0;

        push_camera_state();
        let cam = Camera2D {
            render_target: Some(canvas),
            target: vec2(cam_x, cam_y),
            zoom: vec2(2.0 / WIDTH, 2.0 / HEIGHT) * 16.0,
            ..Default::default()
        };
        set_camera(&cam);

        system_draw_colored_boxes(&self.world, &self.physics);
        system_draw_projectiles(&self.world, &self.physics);
        system_draw_particles(&self.world, &self.physics);

        // just do some debug drawing for now
        if controls.pressed(Control::Debug) {
            system_draw_collision(&self.world, &self.physics);
        }

        // For now, do this terrible O(n^3) nonsense
        let mut lightmap = Image::gen_image_color(
            (WIDTH / 16.0 * LIGHT_RESOLUTION) as u16,
            (HEIGHT / 16.0 * LIGHT_RESOLUTION) as u16,
            BLACK,
        );
        for px in 0..lightmap.width() {
            for py in 0..lightmap.height() {
                let world_pos = self.camera_pos
                    + vec2(
                        // we add 0.5 to center the dots
                        (px as f32 - lightmap.width() as f32 / 2.0 + 0.5) / LIGHT_RESOLUTION,
                        (py as f32 - lightmap.height() as f32 / 2.0 + 0.5) / LIGHT_RESOLUTION,
                    );
                draw_circle(world_pos.x, world_pos.y, 0.1, WHITE);
                let world_point: Point2<f32> = world_pos.into();

                for (e, (coll_h, light)) in self
                    .world
                    .query::<(&HasCollider, &Illuminator)>()
                    .into_iter()
                {
                    let coll = self.physics.colliders.get(**coll_h).unwrap();
                    let lightpos = coll.compute_aabb().center();

                    // Do we try to send light through anything?
                    let delta = world_point - lightpos;
                    let ray = Ray::new(lightpos, delta.normalize());
                    let raycast = self.physics.query_pipeline.cast_ray(
                        &self.physics.colliders,
                        &ray,
                        Real::MAX,
                        false,
                        InteractionGroups::new(
                            collider_groups::GROUP_LIGHTING,
                            collider_groups::FILTER_LIGHTING,
                        ),
                        None,
                    );
                    if raycast.is_none() || controls.pressed(Control::Debug) {
                        // didn't hit anything on the way
                        let color = light.get_color(lightpos.into(), world_pos);
                        let existing = lightmap.get_pixel(px as u32, py as u32).to_vec();
                        lightmap.set_pixel(
                            px as u32,
                            py as u32,
                            Color::from_vec(color.extend(1.0) + existing),
                        );
                    }
                }
            }
        }

        pop_camera_state();

        self.light_tex.update(&lightmap);
        assets
            .shaders
            .lighting
            .set_texture("lights", self.light_tex);
        gl_use_material(assets.shaders.lighting);
        draw_texture(canvas.texture, 0.0, 0.0, WHITE);
        gl_use_default_material();

        system_draw_spellcaster(&self.world, controls);
    }
}

trait WorldExt {
    /// Get the player's ID (specifically, the first Entity with a Player component).
    fn get_player(&self) -> Option<Entity>;

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
    fn get_player(&self) -> Option<Entity> {
        let mut query = self.query::<&Player>();
        query.iter().next().map(|(e, _)| e)
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
