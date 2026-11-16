// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_draw(game: &crate::game::Game<'_>, block: &str, data: &[i32], batch: &mut horizon_client::hud::HudBatch, origin: [f32; 2]) {
    crate::game::ironfurnaces::draw(game, block, data, batch, origin);
}
