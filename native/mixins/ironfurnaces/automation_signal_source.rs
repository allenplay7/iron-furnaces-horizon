// SPDX-License-Identifier: Apache-2.0
// HEAD GameLevel::native_is_signal_source.
fn ironfurnaces_signal_source(level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId, source: &mut bool) {
    if level.registry.state(state).is_some_and(|s|
        crate::game::iron_furnaces_rules::Tier::from_block(&s.name).is_some()) { *source = true; }
}
