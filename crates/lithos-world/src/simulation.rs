//! The game simulation — ties together the ECS world, schedule, and resources.

use bevy_ecs::prelude::*;

use crate::resources::*;
use crate::systems;
use crate::tilemap::TileMap;

/// The game simulation. Wraps a bevy_ecs [`World`] and [`Schedule`].
pub struct Simulation {
    pub world: World,
    schedule: Schedule,
}

impl Simulation {
    /// Create a new simulation with default configuration.
    pub fn new() -> Self {
        Self::with_config(SimConfig::default())
    }

    /// Create a new simulation with custom configuration.
    pub fn with_config(config: SimConfig) -> Self {
        let mut world = World::new();

        // Insert resources.
        let world_seed = config.world_seed;
        world.insert_resource(config);
        world.insert_resource(TickCounter::default());
        world.insert_resource(InputQueue::default());
        world.insert_resource(LastProcessedSeq::default());
        world.insert_resource(EntityRegistry::default());
        world.insert_resource(ZoneChangeEvents::default());
        world.insert_resource(CombatEvents::default());
        world.insert_resource(ChatEvents::default());
        world.insert_resource(ActiveDynamicEvents::default());
        world.insert_resource(DynamicEventBus::default());
        world.insert_resource(RaidStateStore::default());
        world.insert_resource(RaidEventBus::default());
        world.insert_resource(ProgressionQueue::default());
        world.insert_resource(TraderMarket::default());
        world.insert_resource(FactionVaults::default());
        world.insert_resource(MineQueue::default());
        world.insert_resource(MiningEvents::default());
        world.insert_resource(TradeQueue::default());
        world.insert_resource(TradeEvents::default());
        world.insert_resource(LoadedZones::default());
        world.insert_resource(TileMap::new(world_seed));

        // Build the per-tick schedule.
        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                systems::tick_counter_system,
                systems::process_inputs_system,
                systems::combat_system,
                systems::respawn_system,
                systems::position_history_system,
                systems::movement_system,
                systems::bounds_system,
                systems::power_grid_system,
                systems::life_support_system,
                systems::hit_detection_system,
                systems::projectile_expiration_system,
                systems::zone_transfer_system,
                systems::item_pickup_system,
                systems::npc_pathfinding_brain_system,
                systems::npc_ai_system,
            )
                .chain(),
        );
        schedule.add_systems(
            (
                systems::npc_pathfinding_system,
                systems::npc_attack_system,
                systems::trader_market_system,
                systems::mining_system,
                systems::trade_system,
                systems::progression_system,
                systems::dynamic_events_system,
                systems::raid_state_system,
            )
                .chain()
                .after(systems::npc_ai_system),
        );

        Self { world, schedule }
    }

    /// Run one tick of the simulation.
    pub fn tick(&mut self) {
        self.schedule.run(&mut self.world);
    }

    /// Get the current tick number.
    pub fn current_tick(&self) -> u64 {
        self.world.resource::<TickCounter>().tick
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}
