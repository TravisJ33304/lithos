//! Crafting system — recipe definitions and lookup.

use lithos_protocol::SkillBranch;

/// A crafting recipe that transforms input items into an output item.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// Human-readable name of the recipe.
    pub name: &'static str,
    /// Items consumed by the recipe.
    pub inputs: &'static [&'static str],
    /// Item produced by the recipe.
    pub output: &'static str,
    /// Skill branch required to craft this recipe.
    pub required_branch: SkillBranch,
    /// Minimum level in that branch to unlock this recipe.
    pub required_level: u32,
}

/// All available crafting recipes.
pub static RECIPES: &[Recipe] = &[
    // ── Basic recipes (OuterRim resources) — Fabrication Level 1 ──────────────
    Recipe {
        name: "iron_plate",
        inputs: &["iron", "iron"],
        output: "iron_plate",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "copper_wire",
        inputs: &["copper", "copper"],
        output: "copper_wire",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "circuit",
        inputs: &["copper_wire", "iron_plate"],
        output: "circuit",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "glass",
        inputs: &["silica", "silica"],
        output: "glass",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "medkit",
        inputs: &["biomass", "glass"],
        output: "medkit",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "bio_fuel",
        inputs: &["biomass", "biomass"],
        output: "bio_fuel",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    // ── Mid-tier recipes (MidZone resources) — Fabrication Level 3 ────────────
    Recipe {
        name: "titanium_plate",
        inputs: &["titanium", "titanium"],
        output: "titanium_plate",
        required_branch: SkillBranch::Fabrication,
        required_level: 3,
    },
    Recipe {
        name: "battery",
        inputs: &["titanium_plate", "circuit"],
        output: "battery",
        required_branch: SkillBranch::Fabrication,
        required_level: 3,
    },
    Recipe {
        name: "shield_module",
        inputs: &["titanium_plate", "battery", "circuit"],
        output: "shield_module",
        required_branch: SkillBranch::Fabrication,
        required_level: 3,
    },
    // ── End-game recipes (Core resources) — Fabrication Level 5 ───────────────
    Recipe {
        name: "uranium_core",
        inputs: &["uranium", "uranium"],
        output: "uranium_core",
        required_branch: SkillBranch::Fabrication,
        required_level: 5,
    },
    Recipe {
        name: "plutonium_core",
        inputs: &["plutonium", "plutonium"],
        output: "plutonium_core",
        required_branch: SkillBranch::Fabrication,
        required_level: 5,
    },
    Recipe {
        name: "warp_drive",
        inputs: &["uranium_core", "battery", "titanium_plate"],
        output: "warp_drive",
        required_branch: SkillBranch::Fabrication,
        required_level: 5,
    },
    Recipe {
        name: "breach_generator",
        inputs: &["plutonium_core", "shield_module", "warp_drive"],
        output: "breach_generator",
        required_branch: SkillBranch::Fabrication,
        required_level: 5,
    },
    // ── Base building recipes — Fabrication Level 1-2 ─────────────────────────
    Recipe {
        name: "wall_segment",
        inputs: &["iron_plate", "iron_plate"],
        output: "wall_segment",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "door",
        inputs: &["iron_plate", "circuit"],
        output: "door",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
    Recipe {
        name: "generator",
        inputs: &["battery", "titanium_plate", "circuit"],
        output: "generator",
        required_branch: SkillBranch::Fabrication,
        required_level: 2,
    },
    Recipe {
        name: "workbench",
        inputs: &["iron_plate", "iron_plate", "circuit"],
        output: "workbench",
        required_branch: SkillBranch::Fabrication,
        required_level: 1,
    },
];

/// Look up a recipe by its unique name.
pub fn find_recipe(name: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.name == name)
}
