fn ironfurnaces_ui_item_tooltip(game: &crate::game::Game<'_>, stack: &horizon_client::items::ItemStack, lines: &mut Vec<(horizon_client::text::Text,u32)>) {
    crate::game::ironfurnaces_ui::item_tooltip(game,stack,lines);
}
