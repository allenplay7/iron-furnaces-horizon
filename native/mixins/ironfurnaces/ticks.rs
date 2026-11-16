// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_ticks(block: &str, ticks: &mut bool) {
    if block == "ironfurnaces:heater" || crate::game::iron_furnaces_rules::Tier::from_block(block).is_some() { *ticks = true; }
}
