// SPDX-License-Identifier: Apache-2.0
// HEAD GameLevel::native_block_signal; every face has the configured output.
fn ironfurnaces_signal(level: &crate::game::GameLevel<'_>,
    pos: horizon_core::position::BlockPos, signal: &mut i32) {
    if let Some(horizon_client::containers::BlockEntity::Mod(entity)) =
        level.world.block_entities.get(&(pos.x,pos.y,pos.z))
        && crate::game::iron_furnaces_rules::Tier::from_block(&entity.id).is_some()
    { *signal = crate::game::ironfurnaces_automation::emitted_signal(entity, level.items); }
}
