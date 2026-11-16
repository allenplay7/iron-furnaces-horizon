fn ironfurnaces_logout(game:&mut crate::game::Game<'_>,player:u128) {
    crate::game::ironfurnaces_power::release_owner_chunks(game,player);
}
