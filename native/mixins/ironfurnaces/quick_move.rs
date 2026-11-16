fn ironfurnaces_quick_move(game: &mut crate::game::Game<'_>, entity: &mut horizon_client::containers::ModBlockEntity, container_slots: usize, slot: usize, stack: &mut horizon_client::items::ItemStack, handled: &mut bool) {
    crate::game::ironfurnaces::quick_move(game, entity, container_slots, slot, stack, handled);
}
