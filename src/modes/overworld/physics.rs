use rapier2d::prelude::*;

/// Rapier physics-inator. Everything that is in the game world
/// goes through this (that is to say, everything).
///
/// Units:
/// - 16 pixels equals one meter equals one square.
/// - One frame equals one physics step probably equals 1/60 seconds.
///   On each frame we set the physics step time to however long the last frame took.
///
/// In the Sets, userdata is set to the entity, to bits.
/// This is so you can get an entity from here.
pub struct PhysicsWorld {
    pub rigid_bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub joints: JointSet,

    pub integration_params: IntegrationParameters,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,

    pub physics_pipeline: PhysicsPipeline,
    pub query_pipeline: QueryPipeline,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            rigid_bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: JointSet::new(),

            integration_params: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),

            physics_pipeline: PhysicsPipeline::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }
}

/// Groups for colliders.
///
/// Filters are "Default" filters; you're free to make your own.
pub mod groups {
    /// Walls, decals, and other immobile geometry.
    pub const GROUP_WALLS: u32 = 0x00000001;
    /// Players and enemies.
    pub const GROUP_ANIMATE: u32 = 0x00000002;
    /// Things that should damage but not bounce off of animate things.
    pub const GROUP_PROJECTILES: u32 = 0x00000004;

    /// Things that can interact with walls.
    pub const FILTER_WALLS: u32 = GROUP_ANIMATE | GROUP_PROJECTILES;
    pub const FILTER_ANIMATE: u32 = GROUP_WALLS | GROUP_ANIMATE;
    pub const FILTER_PROJECTILES: u32 = GROUP_WALLS | GROUP_PROJECTILES;
}
