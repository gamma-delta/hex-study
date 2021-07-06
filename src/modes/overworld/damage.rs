use enum_map::EnumMap;

/// Sources of damage
#[derive(Debug, Clone, Copy)]
pub enum DamageSource {
    /// Normal damage from plain spells
    Projectile,
    /// Various explosions
    Explosive,
}

/// One instance of an attack.
///
/// Attacks that do different kinds of damage use multiple damage packets.
#[derive(Debug, Clone, Copy)]
pub struct DamagePacket {
    pub source: DamageSource,
    pub amount: u64,
}

/// A set of resistances (or vulnerabilities!) to damage.
#[derive(Debug, Clone)]
pub struct DamageSensitivities(pub EnumMap<DamageSource, f32>);
