fn ironfurnaces_explosion(game:&mut crate::game::Game<'_>,pos:[i32;3]) -> core::ops::ControlFlow<Result<()>> {
    crate::game::ironfurnaces_extras::explosion(game,pos)
}
