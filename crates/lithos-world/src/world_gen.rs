use noise::{NoiseFn, Perlin};
use lithos_protocol::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    OuterRim,
    MidZone,
    Core,
}

pub struct WorldGenerator {
    #[allow(dead_code)]
    seed: u32,
    noise: Perlin,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            noise: Perlin::new(seed),
        }
    }

    /// Determines the biome at a specific world coordinate.
    /// Uses distance from the origin (0,0) as the primary biome driver,
    /// perturbed by Perlin noise to make the boundaries non-circular and natural.
    pub fn get_biome(&self, pos: Vec2) -> Biome {
        // Sample noise to perturb the radius calculation
        // Scale down the coordinates so the noise is smooth over large areas
        let noise_val = self.noise.get([pos.x as f64 / 1000.0, pos.y as f64 / 1000.0]);
        
        // Base distance from center
        let dist = pos.length();
        
        // Perturb the distance by up to +/- 500 units based on noise
        let perturbed_dist = dist as f64 + (noise_val * 500.0);

        if perturbed_dist < 1500.0 {
            Biome::Core
        } else if perturbed_dist < 3500.0 {
            Biome::MidZone
        } else {
            Biome::OuterRim
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_generation() {
        let generator = WorldGenerator::new(12345);
        
        // Origin should be Core
        assert_eq!(generator.get_biome(Vec2::new(0.0, 0.0)), Biome::Core);
        
        // Far away should be OuterRim
        assert_eq!(generator.get_biome(Vec2::new(5000.0, 0.0)), Biome::OuterRim);
    }
}
