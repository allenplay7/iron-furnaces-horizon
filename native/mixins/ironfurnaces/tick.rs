// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_tick(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut horizon_client::containers::ModBlockEntity) {
    crate::game::ironfurnaces::tick(game, pos, entity);
}
