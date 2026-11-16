fn restore(value: (), game: &mut Game<'_>, pos: (i32, i32, i32)) -> () {
    crate::game::ironfurnaces_compat::restored(game, pos);
    value
}
