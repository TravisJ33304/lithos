//! Crafting system — recipe definitions and lookup.

/// A crafting recipe that transforms input items into an output item.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// Human-readable name of the recipe.
    pub name: &'static str,
    /// Items consumed by the recipe.
    pub inputs: &'static [&'static str],
    /// Item produced by the recipe.
    pub output: &'static str,
}

/// All available crafting recipes.
pub static RECIPES: &[Recipe] = &[
    // Basic recipes (OuterRim resources)
    Recipe {
        name: "iron_plate",
        inputs: &["iron", "iron"],
        output: "iron_plate",
    },
    Recipe {
        name: "circuit",
        inputs: &["iron", "iron_plate"],
        output: "circuit",
    },
    Recipe {
        name: "medkit",
        inputs: &["scrap", "circuit"],
        output: "medkit",
    },
    // Mid-tier recipes (MidZone resources)
    Recipe {
        name: "titanium_plate",
        inputs: &["titanium", "titanium"],
        output: "titanium_plate",
    },
    Recipe {
        name: "battery",
        inputs: &["titanium_plate", "circuit"],
        output: "battery",
    },
    Recipe {
        name: "shield_module",
        inputs: &["titanium_plate", "battery", "circuit"],
        output: "shield_module",
    },
    // End-game recipes (Core resources)
    Recipe {
        name: "lithos_core",
        inputs: &["lithos", "lithos"],
        output: "lithos_core",
    },
    Recipe {
        name: "warp_drive",
        inputs: &["lithos_core", "battery", "titanium_plate"],
        output: "warp_drive",
    },
    Recipe {
        name: "breach_generator",
        inputs: &["lithos_core", "shield_module", "warp_drive"],
        output: "breach_generator",
    },
    // Base building recipes
    Recipe {
        name: "wall_segment",
        inputs: &["iron_plate", "iron_plate"],
        output: "wall_segment",
    },
    Recipe {
        name: "door",
        inputs: &["iron_plate", "circuit"],
        output: "door",
    },
    Recipe {
        name: "generator",
        inputs: &["battery", "titanium_plate", "circuit"],
        output: "generator",
    },
    Recipe {
        name: "workbench",
        inputs: &["iron_plate", "iron_plate", "circuit"],
        output: "workbench",
    },
];
