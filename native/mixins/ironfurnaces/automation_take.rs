// SPDX-License-Identifier: Apache-2.0
// RETURN Containers::can_take; bindings include $value and face.
fn ironfurnaces_automation_take(allowed: bool, containers: &Containers<'_>, view: &View,
    slot: usize, stack: &ItemStack, face: horizon_core::geometry::Direction) -> bool {
    if !allowed { return false; }
    if let Some((pos, local)) = view.locate(slot)
        && let Some(BlockEntity::Mod(entity)) = containers.world.block_entities.get(&pos)
        && crate::game::iron_furnaces_rules::Tier::from_block(&entity.id).is_some()
    {
        return crate::game::ironfurnaces_automation::can_extract_through(entity, local, stack, face);
    }
    allowed
}
