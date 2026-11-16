fn ironfurnaces_place(game:&mut crate::game::Game<'_>,stack:&horizon_client::items::ItemStack,pos:[i32;3]) {
    crate::game::ironfurnaces_extras::placed(game,stack,pos);
}
