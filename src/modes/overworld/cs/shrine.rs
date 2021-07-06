/// Component for shrines. These are what bring you between levels.
pub struct Shrine {
    /// The level of the map this shrine leads to.
    level: u64,
}

impl Shrine {
    pub fn new(level: u64) -> Self {
        Self { level }
    }
}
