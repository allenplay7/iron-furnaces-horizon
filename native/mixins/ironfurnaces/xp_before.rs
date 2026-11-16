// SPDX-License-Identifier: Apache-2.0
fn iron_furnaces_xp_before(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, packet: &horizon_protocol::packets::play::Serverbound) {
    // Packet handling and client prediction use separate threads. Keep an
    // authoritative context only until the next packet or completed tick.
    horizon_client::menu::ironfurnaces_slots::clear_context();
    crate::game::ironfurnaces::packet_before(game, host, conn, packet);
}
