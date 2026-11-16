// SPDX-License-Identifier: Apache-2.0
// Owner placement indexes retain their 0.2 save format; tickets use the new API.
use crate::{
    config,
    energy_rules::{self, GeneratorPresence, MachineMode},
    inventory,
    rules::Tier,
    state::*,
};
use mod_api::{
    BlockSide, Compound, Host, LevelRef, ModBlockEntityView, ServerHost, Tag, TicketController,
    TicketOwner,
};
use std::sync::OnceLock;
pub static RAINBOW: OnceLock<TicketController> = OnceLock::new();
pub static WIRELESS: OnceLock<TicketController> = OnceLock::new();
const INDEX: &str = "ironfurnaces:furnaces";
pub fn owner(nbt: &Compound) -> Option<u128> {
    let s = string(nbt, "Owner")?;
    if !s.contains('-') {
        if let Ok(n) = s.parse() {
            return Some(n);
        }
    }
    u128::from_str_radix(&s.replace('-', ""), 16).ok()
}
pub fn track(host: &mut dyn ServerHost, pos: [i32; 3], owner: u128) {
    let Some(player) = host.player(owner) else {
        return;
    };
    let Some(mut data) = player.persistent_data(host) else {
        return;
    };
    let entry = data
        .entry(INDEX.into())
        .or_insert_with(|| Tag::Compound(Compound::new()));
    let Tag::Compound(levels) = entry else {
        return;
    };
    let entry = levels
        .entry(host.dimension().into())
        .or_insert_with(|| Tag::IntArray(Vec::new()));
    let Tag::IntArray(positions) = entry else {
        return;
    };
    if positions.chunks_exact(3).any(|p| p == pos) {
        return;
    }
    positions.extend_from_slice(&pos);
    let _ = player.set_persistent_data(host, data);
}
fn positions(host: &dyn Host, owner: u128) -> Vec<[i32; 3]> {
    let Some(player) = host.player(owner) else {
        return Vec::new();
    };
    let Some(data) = player.persistent_data(host) else {
        return Vec::new();
    };
    let Some(Tag::Compound(levels)) = data.get(&(INDEX).into()) else {
        return Vec::new();
    };
    let Some(Tag::IntArray(points)) = levels.get(&(host.dimension()).into()) else {
        return Vec::new();
    };
    points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
}
pub fn release(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: TicketController,
    owner: TicketOwner,
) {
    for (who, pos, natural) in level
        .mod_forced_chunks(host, controller)
        .unwrap_or_default()
    {
        if who == owner {
            if let Some(chunk) = level.chunk_ref(host, pos) {
                chunk.set_mod_forced(host, controller, who, false, natural);
            }
        }
    }
}
pub fn wireless_load(
    host: &mut dyn ServerHost,
    level: LevelRef,
    consumer: [i32; 3],
    source: [i32; 3],
) {
    let Some(&controller) = WIRELESS.get() else {
        return;
    };
    let wanted = [source[0] >> 4, source[2] >> 4];
    let owner = TicketOwner::Block(consumer);
    for (who, pos, natural) in level
        .mod_forced_chunks(host, controller)
        .unwrap_or_default()
    {
        if who == owner && pos != wanted {
            if let Some(c) = level.chunk_ref(host, pos) {
                c.set_mod_forced(host, controller, who, false, natural);
            }
        }
    }
    if let Some(c) = level.chunk_ref(host, wanted) {
        c.set_mod_forced(host, controller, owner, true, false);
    }
}
pub fn ready(host: &mut dyn ServerHost, pos: [i32; 3], e: &ModBlockEntityView<'_>) -> bool {
    let Some(who) = owner(&e.nbt) else {
        return false;
    };
    let Some(level) = host.level(host.dimension()) else {
        return false;
    };
    let online = level
        .online_players(host)
        .unwrap_or_default()
        .iter()
        .any(|p| p.uuid() == who);
    if !online {
        return false;
    }
    track(host, pos, who);
    let now = level.game_time(host).unwrap_or(0);
    let mut count = 1;
    let mut generators = Vec::new();
    for point in positions(host, who) {
        if let Some(&controller) = RAINBOW.get() {
            if let Some(c) = level.chunk_ref(host, [point[0] >> 4, point[2] >> 4]) {
                c.set_mod_forced(host, controller, TicketOwner::Entity(who), true, false);
            }
        }
        if point == pos {
            continue;
        }
        let Some((_, mut other)) = inventory::snapshot(
            host,
            BlockSide {
                level,
                pos: point,
                side: None,
            },
        ) else {
            continue;
        };
        let other = ModBlockEntityView::new(&mut other);
        if owner(&other.nbt) != Some(who) || mode(host, &other) != MachineMode::Generator {
            continue;
        }
        let Some(tier) = Tier::from_block(other.id()) else {
            continue;
        };
        if tier == Tier::Rainbow {
            count += 1;
        }
        let fresh = matches!(other.nbt.get(&("GeneratorLastTick").into()),Some(Tag::Long(t)) if *t>=now.saturating_sub(1));
        generators.push(GeneratorPresence {
            tier,
            running: fresh && number(&other.nbt, "GeneratorRemaining") > 0,
            energy: number(&other.nbt, "energy").max(0) as u32,
            capacity: config::energy_capacity(tier),
        });
    }
    energy_rules::rainbow_ready(online, count, &generators)
}
pub fn maintenance(host: &mut dyn ServerHost, level: LevelRef) {
    if let Some(&controller) = WIRELESS.get() {
        let now = level.game_time(host).unwrap_or(0);
        for (who, pos, natural) in level
            .mod_forced_chunks(host, controller)
            .unwrap_or_default()
        {
            let valid = if let TicketOwner::Block(point) = who {
                host.block_entity(level,point).and_then(|b|b.data(host))
                .is_some_and(|d|matches!(d.get(&("WirelessLoadTick").into()),Some(Tag::Long(t)) if *t>=now.saturating_sub(1)))
            } else {
                false
            };
            if !valid {
                if let Some(c) = level.chunk_ref(host, pos) {
                    c.set_mod_forced(host, controller, who, false, natural);
                }
            }
        }
    }
    if let Some(&controller) = RAINBOW.get() {
        let owners: std::collections::BTreeSet<_> = level
            .mod_forced_chunks(host, controller)
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.0)
            .collect();
        for who in owners {
            let valid = if let TicketOwner::Entity(uuid) = who {
                level
                    .online_players(host)
                    .unwrap_or_default()
                    .iter()
                    .any(|p| p.uuid() == uuid)
                    && positions(host, uuid).into_iter().any(|p| {
                        inventory::snapshot(
                            host,
                            BlockSide {
                                level,
                                pos: p,
                                side: None,
                            },
                        )
                        .is_some_and(|(_, mut e)| {
                            let e = ModBlockEntityView::new(&mut e);
                            e.id() == Tier::Rainbow.block_id()
                                && owner(&e.nbt) == Some(uuid)
                                && mode(host, &e) == MachineMode::Generator
                        })
                    })
            } else {
                false
            };
            if !valid {
                release(host, level, controller, who);
            }
        }
    }
}
