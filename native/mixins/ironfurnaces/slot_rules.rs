fn ironfurnaces_slot_rules(menu: &str, slot: usize, stack: &horizon_client::items::ItemStack, may_place: &mut bool, max_stack: &mut u32) {
    if !crate::menu::ironfurnaces_slots::may_place(menu, slot, &stack.item) { *may_place = false; }
    if let Some(limit) = crate::menu::ironfurnaces_slots::slot_limit(menu, slot, &stack.item) {
        *max_stack = (*max_stack).min(limit);
    }
}
