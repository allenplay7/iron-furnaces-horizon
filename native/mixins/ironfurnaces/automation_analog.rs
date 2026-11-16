// SPDX-License-Identifier: Apache-2.0
// HEAD GameLevel::native_analog_output. Independent of active redstone mode.
fn ironfurnaces_analog(level: &crate::game::GameLevel<'_>,
    pos: horizon_core::position::BlockPos, output: &mut i32) {
    if let Some(horizon_client::containers::BlockEntity::Mod(entity)) =
        level.world.block_entities.get(&(pos.x,pos.y,pos.z))
        && crate::game::iron_furnaces_rules::Tier::from_block(&entity.id).is_some()
    { *output = crate::game::ironfurnaces_automation::comparator(entity, level.items); }
}
