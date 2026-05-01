//! Shared item and recipe metadata exposed to clients.

use lithos_protocol::{ItemCategory, ItemDefinition, ItemRarity, RecipeDefinition};

use crate::crafting::RECIPES;

pub fn item_definitions() -> Vec<ItemDefinition> {
    vec![
        item(
            "iron",
            "Iron Ore",
            "Basic construction ore.",
            ItemCategory::Resource,
        ),
        item(
            "copper",
            "Copper Ore",
            "Conductive crafting ore.",
            ItemCategory::Resource,
        ),
        item(
            "silica",
            "Silica",
            "Refined into glass.",
            ItemCategory::Resource,
        ),
        item(
            "uranium",
            "Uranium",
            "High energy core material.",
            ItemCategory::Resource,
        ),
        item(
            "plutonium",
            "Plutonium",
            "End-game fissile material.",
            ItemCategory::Resource,
        ),
        item(
            "biomass",
            "Biomass",
            "Organic crafting input.",
            ItemCategory::Resource,
        ),
        item(
            "mining_laser",
            "Mining Laser",
            "Standard extraction tool.",
            ItemCategory::Tool,
        ),
        item(
            "plasma_pistol",
            "Plasma Pistol",
            "Fallback sidearm.",
            ItemCategory::Weapon,
        ),
        item(
            "medkit",
            "Medkit",
            "Restores health on use.",
            ItemCategory::Consumable,
        ),
        item(
            "breach_generator",
            "Breach Generator",
            "Initiates raid breach workflow.",
            ItemCategory::Utility,
        ),
    ]
}

pub fn recipe_definitions() -> Vec<RecipeDefinition> {
    RECIPES
        .iter()
        .map(|recipe| RecipeDefinition {
            name: recipe.name.to_string(),
            output: recipe.output.to_string(),
            required_branch: recipe.required_branch,
            required_level: recipe.required_level,
            inputs: recipe
                .inputs
                .iter()
                .map(|input| (*input).to_string())
                .collect(),
        })
        .collect()
}

fn item(item: &str, display: &str, description: &str, category: ItemCategory) -> ItemDefinition {
    ItemDefinition {
        item: item.to_string(),
        display_name: display.to_string(),
        description: description.to_string(),
        rarity: ItemRarity::Common,
        category,
        stack_limit: 999,
    }
}
