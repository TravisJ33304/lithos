//! Economy models used by world and server simulation loops.

use lithos_protocol::TraderQuote;

/// Runtime supply/demand state for one trader + item pair.
#[derive(Debug, Clone)]
pub struct TraderMarketState {
    pub trader_entity_id: lithos_protocol::EntityId,
    pub item: String,
    pub base_price: f32,
    pub demand_scalar: f32,
    pub available_credits: i64,
    pub daily_credit_limit: i64,
    pub daily_credits_used: i64,
}

impl TraderMarketState {
    /// Convert market state into a public quote payload.
    pub fn as_quote(&self) -> TraderQuote {
        let buy = (self.base_price * self.demand_scalar).max(1.0);
        let sell = (buy * 0.75).max(1.0);
        TraderQuote {
            trader_entity_id: self.trader_entity_id,
            item: self.item.clone(),
            buy_price: buy,
            sell_price: sell,
            demand_scalar: self.demand_scalar,
            available_credits: self.available_credits,
            daily_credit_limit: self.daily_credit_limit,
            daily_credits_used: self.daily_credits_used,
        }
    }

    /// Update demand scalar from daily net volume.
    pub fn apply_daily_volume(&mut self, sold_to_trader: i32, bought_from_trader: i32) {
        let pressure = (bought_from_trader - sold_to_trader) as f32 * 0.03;
        self.demand_scalar = (self.demand_scalar + pressure).clamp(0.4, 2.2);
    }
}
