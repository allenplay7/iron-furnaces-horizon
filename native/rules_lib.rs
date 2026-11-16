// The same pure modules are embedded in Horizon's game module by the manifest.
#[path = "mixins/ironfurnaces/runtime_rules.rs"]
pub mod iron_furnaces_rules;
#[path = "mixins/ironfurnaces/energy_rules.rs"]
pub mod iron_furnaces_energy_rules;
#[cfg(test)]
#[path = "mixins/ironfurnaces/slot_state.rs"]
mod iron_furnaces_slot_rules;

pub use iron_furnaces_rules::*;
