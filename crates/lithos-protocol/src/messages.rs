//! Client and server message definitions for the Lithos protocol.

use serde::{Deserialize, Serialize};

use crate::types::{
    ChatChannel, DynamicEventSnapshot, EntityId, EntitySnapshot, InteractableSnapshot,
    InventorySnapshot, ItemDefinition, PlayerId, PowerNetworkSnapshot, ProgressionSnapshot,
    RaidStateSnapshot, RecipeDefinition, SkillBranch, TraderQuote, Vec2, ZoneId,
};

// ---------------------------------------------------------------------------
// Client → Server
// ---------------------------------------------------------------------------

/// Messages sent from the game client to the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMessage {
    /// Request to join the game with a given auth token.
    Join {
        /// Supabase JWT token.
        token: String,
    },

    /// Movement input vector (normalised direction).
    Move {
        /// The direction the player intends to move.
        direction: Vec2,
        /// Client-side sequence number for reconciliation.
        seq: u32,
    },

    /// Request to transfer to a different zone.
    ZoneTransfer {
        /// Target zone.
        target: ZoneId,
    },

    /// Request to fire equipped weapon in a specific direction.
    Fire {
        /// The target direction vector.
        direction: Vec2,
        /// Client-measured round trip latency in milliseconds.
        ///
        /// The server uses this hint to rewind state for hit registration.
        client_latency_ms: u16,
    },

    /// Request to respawn after dying.
    Respawn,

    /// Request to craft an item using a named recipe.
    Craft {
        /// The recipe name to craft.
        recipe: String,
    },

    /// Request to build a structure on a grid.
    BuildStructure {
        /// The structure item to build (e.g. "wall_segment").
        item: String,
        /// X coordinate on the grid.
        grid_x: i32,
        /// Y coordinate on the grid.
        grid_y: i32,
    },

    /// Periodic heartbeat / keep-alive.
    Ping {
        /// Client timestamp (ms since epoch).
        timestamp: u64,
    },

    /// Send a chat message.
    Chat {
        /// Channel scope.
        channel: ChatChannel,
        /// Message body.
        text: String,
    },

    /// Ask the server for the current trader quotes.
    RequestTraderQuotes,

    /// Attempt to initiate a raid against a defender faction.
    InitiateRaid { defender_faction_id: u64 },

    /// Request to mine a resource node.
    Mine {
        /// Optional explicit target. If None, server auto-targets nearest node.
        target_entity_id: Option<EntityId>,
    },

    /// Sell an item from inventory to the nearest trader.
    SellItem {
        /// Item type to sell (e.g. "iron").
        item: String,
        /// Quantity to sell.
        quantity: u32,
    },

    /// Buy an item from the nearest trader.
    BuyItem {
        /// Item type to buy (e.g. "medkit").
        item: String,
        /// Quantity to buy.
        quantity: u32,
    },

    /// Context interaction (right click / use key) against a nearby target.
    Interact {
        /// Optional explicit interaction target.
        target_entity_id: Option<EntityId>,
    },

    /// Alternate fire mode for equipped weapon/tool.
    AltFire {
        /// The target direction vector.
        direction: Vec2,
        /// Client-measured round trip latency in milliseconds.
        client_latency_ms: u16,
    },

    /// Drop an inventory item stack into the world.
    DropItem { item: String, quantity: u32 },

    /// Use an item from inventory.
    UseItem { item: String },

    /// Equip an item to a named slot.
    EquipItem { item: String, slot: String },

    /// Request full crafting catalog and player unlock state.
    RequestCraftingState,

    /// Request current power networks for the active zone.
    RequestPowerState,

    /// Begin a timed hack interaction.
    StartHack { target_entity_id: EntityId },

    /// Cancel active hack interaction.
    CancelHack,

    /// Query online factions that can currently be raided.
    RequestRaidTargets,
}

// ---------------------------------------------------------------------------
// Server → Client
// ---------------------------------------------------------------------------

/// Messages sent from the server to game clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    /// Acknowledgement of a successful join. Provides the client with its IDs.
    JoinAck {
        player_id: PlayerId,
        entity_id: EntityId,
        zone: ZoneId,
        world_seed: u32,
    },

    /// A full or delta state snapshot of all visible entities.
    StateSnapshot {
        /// Server tick number.
        tick: u64,
        /// Last client input sequence the server has processed.
        last_processed_seq: u32,
        /// Visible entity states.
        entities: Vec<EntitySnapshot>,
    },

    /// Notification that the player has been transferred to a new zone.
    ZoneChanged { zone: ZoneId },

    /// Notification that an entity's health changed.
    HealthChanged {
        entity_id: EntityId,
        health: f32,
        max_health: f32,
    },

    /// Notification that a player's oxygen level changed.
    OxygenChanged {
        entity_id: EntityId,
        current: f32,
        max: f32,
    },

    /// Notification that a player has died.
    PlayerDied { entity_id: EntityId },

    /// Notification that a player's inventory was updated.
    InventoryUpdated {
        entity_id: EntityId,
        // Using a generic string for now to represent serialized inventory data
        items_json: String,
    },

    /// Structured inventory update payload for UI migration.
    InventorySnapshot { inventory: InventorySnapshot },

    /// Notification that a projectile was spawned. Useful for client-side VFX.
    SpawnProjectile {
        entity_id: EntityId,
        position: Vec2,
        velocity: Vec2,
    },

    /// Broadcast chat line.
    ChatMessage {
        from_entity_id: EntityId,
        channel: ChatChannel,
        text: String,
        sent_at_unix_ms: u64,
    },

    /// Credits / faction vault update.
    CreditsChanged { faction_id: u64, balance: i64 },

    /// Updated NPC trader market quotes.
    TraderQuotes { quotes: Vec<TraderQuote> },

    /// Full item and recipe metadata catalog.
    CraftingCatalog {
        items: Vec<ItemDefinition>,
        recipes: Vec<RecipeDefinition>,
    },

    /// Updated progression stats for a player.
    ProgressionUpdated {
        entity_id: EntityId,
        branches: Vec<ProgressionSnapshot>,
    },

    /// A dynamic world event started.
    DynamicEventStarted { event: DynamicEventSnapshot },

    /// A dynamic world event ended.
    DynamicEventEnded { event_id: u64 },

    /// Nearby interactable focus and requirements.
    InteractableUpdated { interactable: InteractableSnapshot },

    /// Defender warning for an incoming breach.
    RaidWarning { raid: RaidStateSnapshot },

    /// Breach became active.
    RaidStarted { raid: RaidStateSnapshot },

    /// Breach ended.
    RaidEnded {
        raid: RaidStateSnapshot,
        attacker_won: bool,
    },

    /// Response to a client ping.
    Pong {
        /// Echoed client timestamp.
        client_timestamp: u64,
        /// Server timestamp (ms since epoch).
        server_timestamp: u64,
    },

    /// A chunk of the overworld tilemap.
    WorldMapChunk {
        chunk_x: i32,
        chunk_y: i32,
        tiles: Vec<crate::types::TileData>,
    },

    /// A resource node was depleted and will despawn.
    ResourceDepleted { entity_id: EntityId },

    /// The player gained XP in a skill branch.
    XpGained {
        branch: SkillBranch,
        amount: u32,
        new_total: u32,
        new_level: u32,
    },

    /// A crafting request was denied.
    CraftDenied { reason: String },

    /// A trade transaction failed.
    TradeFailed { reason: String },

    /// Weapon ammo count changed.
    AmmoChanged {
        entity_id: EntityId,
        ammo: u32,
        max_ammo: u32,
    },

    /// Power network summary for the player's current zone.
    PowerState {
        zone: ZoneId,
        networks: Vec<PowerNetworkSnapshot>,
    },

    /// Online factions currently available as raid targets.
    RaidTargets { defender_faction_ids: Vec<u64> },

    /// The server is kicking the client.
    Disconnect { reason: String },
}
