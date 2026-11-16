// SPDX-License-Identifier: Apache-2.0
// HEAD GameLevel::native_has_analog_output.
fn ironfurnaces_has_analog(level: &crate::game::GameLevel<'_>,
    state: horizon_world::registry::BlockStateId, output: &mut bool) {
    if level.registry.state(state).is_some_and(|s|
        crate::game::iron_furnaces_rules::Tier::from_block(&s.name).is_some()) { *output = true; }
}
