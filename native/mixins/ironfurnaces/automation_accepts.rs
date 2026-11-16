// SPDX-License-Identifier: Apache-2.0
// HEAD Game::native_container_accepts; replaces the v1 accepts helper.
fn ironfurnaces_automation_accepts(game: &crate::game::Game<'_>,
    entity: &horizon_client::containers::ModBlockEntity, slot: usize,
    stack: &horizon_client::items::ItemStack, face: Option<horizon_core::geometry::Direction>, cancel: &mut bool) {
    if !crate::game::ironfurnaces::accepts(game, entity, slot, stack) { *cancel = true; }
    if crate::game::iron_furnaces_rules::Tier::from_block(&entity.id).is_some()
        && !crate::game::ironfurnaces_automation::can_insert_through(entity, slot, face) { *cancel = true; }
}
