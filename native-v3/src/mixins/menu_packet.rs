fn menu_packet(game: &mut Game<'_>, conn: u32, packet: &S) {
    let S::ContainerClick(click) = packet else {
        return;
    };
    let Some(host) = game.lan.as_mut() else {
        return;
    };
    let Some(gi) = host.hub.guests.iter().position(|g| g.conn == conn) else {
        return;
    };
    let key = host.hub.guests.update(gi, |guest| {
        if guest.spectator() || guest.dead {
            return None;
        }
        let menu = guest
            .menu
            .as_mut()
            .filter(|m| click.container != 0 && m.window == click.container)?;
        let crate::lan::menu::Slots::Block(pos) = menu.slots else {
            return None;
        };
        let Some(crate::containers::BlockEntity::Mod(e)) = game.world.block_entities.get(&pos)
        else {
            return None;
        };
        if !iron_furnaces_horizon::ui::owns(e.id()) {
            return None;
        }
        menu.hidden = iron_furnaces_horizon::ui::hidden(e.id(), &e.data, e.items.len());
        Some(pos)
    });
    if let Some(pos) = key {
        if let Some(menus) = host.menus.get_mut(&conn) {
            *menus.crafting.machine = None;
        }
        if click.slot == 2 || (13..19).contains(&click.slot) || click.input == 6 {
            if let Some(crate::containers::BlockEntity::Mod(e)) =
                game.world.block_entities.get_mut(&pos)
            {
                let count: i32 = [2, 13, 14, 15, 16, 17, 18]
                    .into_iter()
                    .filter_map(|i| e.items.get(i).and_then(Option::as_ref))
                    .map(|s| s.count.min(i32::MAX as u32) as i32)
                    .sum();
                e.nbt.insert(
                    "PendingExtraction".into(),
                    horizon_core::nbt::Tag::Int(count),
                );
            }
        }
    }
}
