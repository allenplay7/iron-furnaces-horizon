// Apache-2.0; see LICENSE and NOTICE. Machine screens use the upstream art.
use super::{Game, iron_furnaces_rules::Tier};
use horizon_client::{containers::ModBlockEntity, hud::HudBatch, items::ItemStack, mod_api::NativeScreen};
use horizon_world::nbt::{Compound, JavaString, Tag};

const HEATER: &str = "ironfurnaces:heater";
const SETTINGS: [&str; 12] = ["Side0", "Side1", "Side2", "Side3", "Side4", "Side5",
    "AutoInput", "AutoOutput", "RedstoneMode", "RedstoneValue", "AugmentTab", "AutoSplit"];
const SIDE_NAMES: [&str; 6] = ["Down", "Up", "North", "South", "West", "East"];
const SIDE_SHORT: [&str; 6] = ["D", "U", "N", "S", "W", "E"];
const SIDE_COLORS: [u32; 5] = [0xff777777, 0xff2679bb, 0xffe38730, 0xff8750b0, 0xffb64f4f];

pub fn owns(block: &str) -> bool { block == HEATER || Tier::from_block(block).is_some() }
fn d(data: &[i32], index: usize) -> i32 { data.get(index).copied().unwrap_or(0) }
fn ratio(data: &[i32], current: usize, total: usize) -> f32 {
    if d(data,total) <= 0 { 0.0 } else { (d(data,current).max(0) as f32 / d(data,total) as f32).clamp(0.0,1.0) }
}
fn wide(data: &[i32], index: usize) -> u32 {
    (d(data,index) as u32 & 0xffff) | ((d(data,index+1) as u32 & 0xffff) << 16)
}
fn generation_rate(data: &[i32]) -> u32 {
    (d(data,36) as u32 & 0xffff) | ((d(data,40) as u32 & 0xffff) << 16)
}
fn lane_visible(tier: i32, lane: usize) -> bool {
    match tier { 0 => (2..=3).contains(&lane), 1 => (1..=4).contains(&lane), _ => lane < 6 }
}

/// All machines retain nineteen backing slots; only the selected page is active.
pub fn layout(block: &str, data: &[i32]) -> NativeScreen {
    let heater = block == HEATER;
    let mut slots = vec![None; if heater {1} else {19}];
    if heater { slots[0] = Some([80.0,37.0]); }
    else if d(data,32) != 0 {
        for (slot,x) in [(3,26.0),(4,80.0),(5,134.0)] { slots[slot] = Some([x,35.0]); }
    } else {
        match d(data,4) {
            1 => for lane in 0..6 {
                if lane_visible(d(data,5),lane) {
                    let x = 28.0 + 21.0 * lane as f32;
                    slots[7+lane] = Some([x,6.0]); slots[13+lane] = Some([x,55.0]);
                }
            },
            2 => slots[6] = Some([56.0,40.0]),
            _ => { slots[0]=Some([56.0,17.0]); slots[1]=Some([56.0,53.0]); slots[2]=Some([116.0,35.0]); }
        }
    }
    NativeScreen { width:if heater {176.0} else {276.0}, height:if heater {166.0} else {184.0},
        slots, inventory_y:84.0, inventory_x:8.0, own_labels:true, own_background:true }
}
pub fn hidden(block: &str, data: &[i32], size: usize) -> Vec<usize> {
    let screen = layout(block,data);
    (0..size).filter(|&slot| screen.slots.get(slot).is_none_or(Option::is_none)).collect()
}
fn inside(at: [f32;2], rect: [f32;4]) -> bool {
    at[0] >= rect[0] && at[1] >= rect[1] && at[0] < rect[0]+rect[2] && at[1] < rect[1]+rect[3]
}
fn side_rect(side: usize) -> [f32;4] { [184.0 + (side%3) as f32*30.0,22.0+(side/3) as f32*18.0,28.0,15.0] }
fn redstone_rect(mode: usize) -> [f32;4] { [184.0+mode as f32*18.0,90.0,16.0,15.0] }
fn tr(game: &Game<'_>, name: &str, fallback: &str) -> String { game.translate(name,fallback) }
fn text(game: &Game<'_>, batch: &mut HudBatch, value: &str, x: f32, y: f32, color: u32) {
    game.native_text(batch,value,[x,y],color,false);
}
fn centered(game: &Game<'_>, batch: &mut HudBatch, value: &str, x: f32, y: f32, width: f32, color: u32) {
    text(game,batch,value,(x+(width-game.native_text_width(value))/2.0).floor(),y,color);
}
fn sprite(game: &Game<'_>, batch: &mut HudBatch, texture: &str, source: [f32;4], origin: [f32;2], rect: [f32;4]) {
    game.native_sprite(batch,texture,source,[origin[0]+rect[0],origin[1]+rect[1],rect[2],rect[3]]);
}
fn button_rect(game: &Game<'_>, batch: &mut HudBatch, origin: [f32;2], rect: [f32;4], label: &str, selected: bool, color: u32) {
    let [x,y,w,h]=rect; let x=x+origin[0]; let y=y+origin[1];
    let cursor=game.native_cursor(); let hovered=inside(cursor,[x,y,w,h]);
    game.native_fill(batch,[x,y,w,h],if hovered {0xffeeeeee} else {0xff373737});
    game.native_fill(batch,[x+1.0,y+1.0,w-2.0,h-2.0],if selected {color} else {0xffaaaaaa});
    centered(game,batch,label,x,y+3.0,w,if selected {0xffffff} else {0x202020});
}
fn furnace_texture(block: &str, generator: bool) -> String {
    let suffix = match Tier::from_block(block) {
        Some(Tier::Netherite)=>"_netherite", Some(Tier::Allthemodium)=>"_allthemodium",
        Some(Tier::Vibranium)=>"_vibranium", Some(Tier::Unobtainium)=>"_unobtainium", _=>"",
    };
    format!("ironfurnaces:gui/furnace{}{suffix}",if generator {"_generator"} else {""})
}
fn energy_label(data: &[i32]) -> String { format!("{} / {} FE",wide(data,6),wide(data,8)) }

pub fn draw(game: &Game<'_>, block: &str, data: &[i32], batch: &mut HudBatch, origin: [f32;2]) {
    if !owns(block) { return; }
    let [x,y]=origin; let heater=block==HEATER; let augment=d(data,32)!=0; let mode=d(data,4);
    let texture=if heater {"ironfurnaces:gui/heater".to_owned()} else if augment {"ironfurnaces:gui/augment".to_owned()}
        else if mode==1 {"ironfurnaces:gui/furnace_factory".to_owned()} else {furnace_texture(block,mode==2)};
    sprite(game,batch,&texture,[0.0,0.0,176.0,166.0],origin,[0.0,0.0,176.0,166.0]);
    let capacity=wide(data,8); let energy=wide(data,6);
    let energy_ratio=if capacity==0 {0.0} else {(energy as f32/capacity as f32).clamp(0.0,1.0)};
    if heater {
        let w=(42.0*energy_ratio).floor();
        if w>0.0 { sprite(game,batch,&texture,[176.0,0.0,w,14.0],origin,[67.0,63.0,w,14.0]); }
        centered(game,batch,&energy_label(data),x,y+21.0,176.0,0x404040);
    } else if !augment {
        match mode {
            1 => {
                for lane in 0..6 {
                    if !lane_visible(d(data,5),lane) { continue; }
                    if lane!=2 && lane!=3 { sprite(game,batch,&texture,[176.0,64.0,18.0,67.0],origin,[27.0+21.0*lane as f32,5.0,18.0,67.0]); }
                    let h=(22.0*ratio(data,10+2*lane,11+2*lane)).floor();
                    if h>0.0 { sprite(game,batch,&texture,[176.0,0.0,15.0,h],origin,[29.0+21.0*lane as f32,27.0,15.0,h]); }
                }
                let h=(42.0*energy_ratio).floor();
                if h>0.0 { sprite(game,batch,&texture,[176.0,64.0-h,14.0,h],origin,[9.0,49.0-h,14.0,h]); }
                button_rect(game,batch,origin,[9.0,56.0,14.0,14.0],"S",d(data,33)!=0,0xff477e42);
            }
            2 => {
                let h=(42.0*energy_ratio).floor();
                if h>0.0 { sprite(game,batch,&texture,[176.0,56.0-h,14.0,h],origin,[109.0,64.0-h,14.0,h]); }
                if d(data,0)>0 { let h=(13.0*ratio(data,0,1)).ceil()+1.0;
                    sprite(game,batch,&texture,[176.0,14.0-h,14.0,h],origin,[56.0,37.0-h,14.0,h]); }
                centered(game,batch,&format!("{} FE/t",generation_rate(data)),x,y+67.0,176.0,0x404040);
            }
            _ => {
                if d(data,0)>0 { let h=(13.0*ratio(data,0,1)).ceil()+1.0;
                    sprite(game,batch,&texture,[176.0,14.0-h,14.0,h],origin,[56.0,50.0-h,14.0,h]); }
                let w=(24.0*ratio(data,2,3)).floor();
                if w>0.0 { sprite(game,batch,&texture,[176.0,14.0,w,16.0],origin,[79.0,34.0,w,16.0]); }
            }
        }
    }
    let title=game.native_screen_title().unwrap_or_else(||block.to_owned());
    centered(game,batch,&title,x,if !heater && mode==1 && !augment {y-10.0} else {y+6.0},176.0,
        if !heater && mode==1 && !augment {0xffffff} else {0x404040});
    text(game,batch,&tr(game,"container.inventory","Inventory"),x+8.0,y+73.0,0x404040);
    if heater { return; }
    button_rect(game,batch,origin,[161.0,4.0,11.0,11.0],if augment {"<"} else {"+"},augment,0xff477e42);
    game.native_fill(batch,[x+180.0,y,96.0,166.0],0xff373737);
    game.native_fill(batch,[x+181.0,y+1.0,94.0,164.0],0xffc6c6c6);
    centered(game,batch,&tr(game,"gui.ironfurnaces.settings","Settings"),x+180.0,y+7.0,96.0,0x404040);
    for side in 0..6 { let setting=d(data,22+side).clamp(0,4) as usize;
        button_rect(game,batch,origin,side_rect(side),SIDE_SHORT[side],true,SIDE_COLORS[setting]); }
    button_rect(game,batch,origin,[184.0,60.0,42.0,15.0],&tr(game,"gui.ironfurnaces.auto_in","Auto In"),d(data,28)!=0,0xff477e42);
    button_rect(game,batch,origin,[230.0,60.0,42.0,15.0],&tr(game,"gui.ironfurnaces.auto_out","Auto Out"),d(data,29)!=0,0xff477e42);
    centered(game,batch,&tr(game,"gui.ironfurnaces.redstone","Redstone"),x+180.0,y+79.0,96.0,0x404040);
    for (mode,label) in ["-","H","L","C","S"].into_iter().enumerate() {
        button_rect(game,batch,origin,redstone_rect(mode),label,d(data,30)==mode as i32,0xffa24646);
    }
    button_rect(game,batch,origin,[184.0,109.0,18.0,15.0],"-",false,0xff777777);
    centered(game,batch,&d(data,31).clamp(0,15).to_string(),x+205.0,y+112.0,46.0,0x404040);
    button_rect(game,batch,origin,[254.0,109.0,18.0,15.0],"+",false,0xff777777);
    button_rect(game,batch,origin,[184.0,129.0,88.0,15.0],&tr(game,"gui.ironfurnaces.reset_sides","Reset sides"),false,0xff777777);
    if mode==1 { button_rect(game,batch,origin,[184.0,147.0,88.0,15.0],&tr(game,"gui.ironfurnaces.auto_split","Auto split"),d(data,33)!=0,0xff477e42); }
    game.native_fill(batch,[x,y+167.0,276.0,17.0],0xffc6c6c6);
    let status=if mode>0 {energy_label(data)} else {
        tr(game,match d(data,34) {1=>"gui.ironfurnaces.smoking",2=>"gui.ironfurnaces.blasting",_=>"gui.ironfurnaces.smelting"},
            match d(data,34) {1=>"Smoking",2=>"Blasting",_=>"Smelting"})
    };
    text(game,batch,&status,x+6.0,y+172.0,0x404040);
    if mode==2 && block=="ironfurnaces:million_furnace" {
        let label=if d(data,37)!=0 {tr(game,"gui.ironfurnaces.rainbow_active","Rainbow active")} else {tr(game,"gui.ironfurnaces.rainbow_waiting","Waiting for generators")};
        text(game,batch,&label,x+276.0-game.native_text_width(&label)-6.0,y+172.0,0x404040);
    }
}

pub fn click(block: &str, data: &[i32], at: [f32;2], right: bool, button: &mut Option<i32>) {
    if !owns(block) || block==HEATER || button.is_some() { return; }
    let found=if inside(at,[161.0,4.0,11.0,11.0]) {Some(0)}
        else if d(data,4)==1 && d(data,32)==0 && inside(at,[9.0,56.0,14.0,14.0]) {Some(3)}
        else if inside(at,[184.0,60.0,42.0,15.0]) {Some(1)}
        else if inside(at,[230.0,60.0,42.0,15.0]) {Some(2)}
        else if inside(at,[184.0,147.0,88.0,15.0]) && d(data,4)==1 {Some(3)}
        else if inside(at,[184.0,109.0,18.0,15.0]) {Some(10)}
        else if inside(at,[254.0,109.0,18.0,15.0]) {Some(11)}
        else if inside(at,[184.0,129.0,88.0,15.0]) {Some(12)}
        else if let Some(side)=(0..6).find(|&side|inside(at,side_rect(side))) {Some(if right {50} else {40}+side as i32)}
        else {(0..5).find(|&mode|inside(at,redstone_rect(mode))).map(|mode|20+mode as i32)};
    *button=found;
}
pub fn tooltip(game: &Game<'_>, block: &str, data: &[i32], at: [f32;2], lines: &mut Vec<String>) {
    if !owns(block) { return; }
    if block==HEATER {
        if inside(at,[67.0,63.0,43.0,14.0]) { lines.push(energy_label(data)); }
        if inside(at,[79.0,36.0,18.0,18.0]) {
            lines.push(tr(game,"gui.ironfurnaces.heater_link","Insert a Wireless Heat Receiver to link it."));
            lines.push(tr(game,"gui.ironfurnaces.heater_use","Then put the receiver in a furnace fuel slot."));
        }
        return;
    }
    if inside(at,[161.0,4.0,11.0,11.0]) { lines.push(tr(game,if d(data,32)==0 {"tooltip.ironfurnaces.gui_open_augments"} else {"tooltip.ironfurnaces.gui_open_furnace"},if d(data,32)==0 {"Augments"} else {"Furnace"})); }
    for side in 0..6 { if inside(at,side_rect(side)) {
        let mode=d(data,22+side).clamp(0,4) as usize;
        let names=[("tooltip.ironfurnaces.gui_none","None"),("tooltip.ironfurnaces.gui_input","Input"),("tooltip.ironfurnaces.gui_output","Output"),("tooltip.ironfurnaces.gui_input_output","Input/Output"),("tooltip.ironfurnaces.gui_fuel","Fuel Input/Eject")];
        lines.push(format!("{}: {}",tr(game,&format!("gui.ironfurnaces.side{}",side),SIDE_NAMES[side]),tr(game,names[mode].0,names[mode].1)));
        lines.push(tr(game,"gui.ironfurnaces.side_cycle","Left click: next; right click: previous"));
    }}
    for mode in 0..5 { if inside(at,redstone_rect(mode)) {
        let labels=[("ignored","Redstone ignored"),("high","Runs with a redstone signal"),("low","Runs without a redstone signal"),("comparator","Comparator output"),("comparator_sub","Comparator with subtraction")];
        lines.push(tr(game,&format!("tooltip.ironfurnaces.gui_redstone_{}",labels[mode].0),labels[mode].1));
    }}
    for (rect,key,fallback) in [([184.0,60.0,42.0,15.0],"tooltip.ironfurnaces.gui_auto_input","Auto input"),
        ([230.0,60.0,42.0,15.0],"tooltip.ironfurnaces.gui_auto_output","Auto output"),
        ([184.0,109.0,88.0,15.0],"gui.ironfurnaces.redstone_value","Comparator subtraction: 0–15"),
        ([184.0,129.0,88.0,15.0],"tooltip.ironfurnaces.gui_reset","Reset all sides"),
        ([184.0,147.0,88.0,15.0],"gui.ironfurnaces.auto_split_hint","Distribute matching input evenly across factory lanes")] {
        if inside(at,rect) && (key!="gui.ironfurnaces.auto_split_hint" || d(data,4)==1) { lines.push(tr(game,key,fallback)); }
    }
    if d(data,4)==1 && d(data,32)==0 && inside(at,[9.0,56.0,14.0,14.0]) {lines.push(tr(game,"gui.ironfurnaces.auto_split_hint","Distribute matching input evenly across factory lanes"));}
    if d(data,32)==0 && ((d(data,4)==1 && inside(at,[9.0,7.0,14.0,42.0])) || (d(data,4)==2 && inside(at,[109.0,22.0,14.0,42.0]))) { lines.push(energy_label(data)); }
}
fn number(nbt: &Compound, key: &str) -> i32 { match nbt.get(&JavaString::from(key)) {Some(Tag::Int(n))=>*n,_=>0} }
fn put(nbt: &mut Compound, key: &str, value: i32) { nbt.insert(key.into(),Tag::Int(value)); }

pub fn item_tooltip(game: &Game<'_>, stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text,u32)>) {
    let mut extra=Vec::new();
    match &*stack.item {
        "ironfurnaces:heater" => {
            let energy=super::ironfurnaces_extras::item_energy(stack).unwrap_or(0);
            extra.push(format!("{} / {} FE",energy,super::ironfurnaces_config::heater_capacity()));
        }
        "ironfurnaces:item_heater" => {
            if let Some(tag)=stack.custom_data().filter(|tag|["HeaterPosX","HeaterPosY","HeaterPosZ"].iter().all(|key|tag.contains_key(&JavaString::from(*key)))) {
                extra.push(format!("{}{}, {}, {}",tr(game,"tooltip.ironfurnaces.heater","Bound to: "),
                    number(tag,"HeaterPosX"),number(tag,"HeaterPosY"),number(tag,"HeaterPosZ")));
            } else { extra.push(tr(game,"tooltip.ironfurnaces.heater_not_bound","Not yet linked to a heat transmitter.")); }
            extra.push(tr(game,"gui.ironfurnaces.heater_link","Insert a Wireless Heat Receiver to link it."));
            extra.push(tr(game,"gui.ironfurnaces.heater_use","Then put the receiver in a furnace fuel slot."));
        }
        "ironfurnaces:item_copy" => {
            let custom=stack.custom_data();
            let copied=custom.and_then(|tag|tag.get(&JavaString::from("ironfurnaces:settings"))).and_then(|tag|match tag {Tag::Compound(map)=>Some(map),_=>None});
            let settings:Vec<i32>=if let Some(copied)=copied {SETTINGS.iter().map(|key|number(copied,key)).collect()}
                else if let Some(Tag::IntArray(values))=custom.and_then(|tag|tag.get(&JavaString::from("settings"))) {values.clone()}
                else {Vec::new()};
            if !settings.is_empty() {
                for side in 0..6 { if let Some(value)=settings.get(side) {
                    let descriptions=[("tooltip.ironfurnaces.gui_none","None"),("tooltip.ironfurnaces.gui_input","Input"),("tooltip.ironfurnaces.gui_output","Output"),("tooltip.ironfurnaces.gui_input_output","Input/Output"),("tooltip.ironfurnaces.gui_fuel","Fuel Input/Eject")];
                    let description=descriptions[(*value).clamp(0,4) as usize];
                    extra.push(format!("{}: {}",tr(game,&format!("gui.ironfurnaces.side{side}"),SIDE_NAMES[side]),tr(game,description.0,description.1)));
                }}
                if settings.len()>=10 {
                    extra.push(format!("{}: {}",tr(game,"tooltip.ironfurnaces.gui_auto_input","Auto Input"),settings[6]!=0));
                    extra.push(format!("{}: {}",tr(game,"tooltip.ironfurnaces.gui_auto_output","Auto Output"),settings[7]!=0));
                    extra.push(format!("{}: {} / {}",tr(game,"gui.ironfurnaces.redstone","Redstone"),settings[8],settings[9]));
                }
                if let Some(Tag::String(facing))=copied.and_then(|tag|tag.get(&JavaString::from("Facing"))) {
                    extra.push(format!("{}: {}",tr(game,"gui.ironfurnaces.facing","Facing"),facing.to_string_lossy()));
                }
            }
            extra.push(tr(game,"gui.ironfurnaces.copy_settings","Right click a furnace to copy its settings."));
            extra.push(tr(game,"gui.ironfurnaces.apply_settings","Sneak and right click to apply copied settings."));
        }
        _ => return,
    }
    lines.extend(extra.into_iter().map(|line|(horizon_client::text::Text::literal(line),horizon_client::hud::rgba(170,170,170,255))));
}

/// Only settings are changed here. Inventory and augmentation remain real slots.
pub fn button(game: &mut Game<'_>, _pos: [i32;3], entity: &mut ModBlockEntity, id: i32, _carried: &mut Option<ItemStack>) {
    if !game.native_on("ironfurnaces_native") || !owns(&entity.id) || entity.id==HEATER { return; }
    let change=match id {
        0=>Some((10,1-number(&entity.nbt,"AugmentTab").clamp(0,1))),
        1=>Some((6,1-number(&entity.nbt,"AutoInput").clamp(0,1))),
        2=>Some((7,1-number(&entity.nbt,"AutoOutput").clamp(0,1))),
        3 if d(&entity.data,4)==1=>Some((11,1-number(&entity.nbt,"AutoSplit").clamp(0,1))),
        10=>Some((9,(number(&entity.nbt,"RedstoneValue").clamp(0,15)-1).max(0))),
        11=>Some((9,(number(&entity.nbt,"RedstoneValue").clamp(0,15)+1).min(15))),
        20..=24=>Some((8,id-20)),
        40..=45=>{let side=(id-40) as usize;Some((side,(number(&entity.nbt,SETTINGS[side]).clamp(0,4)+1)%5))},
        50..=55=>{let side=(id-50) as usize;Some((side,(number(&entity.nbt,SETTINGS[side]).clamp(0,4)+4)%5))},
        _=>None,
    };
    if entity.data.len()<64 { entity.data.resize(64,0); }
    if id==12 { for side in 0..6 {put(&mut entity.nbt,SETTINGS[side],0);entity.data[22+side]=0;} }
    if let Some((setting,value))=change {put(&mut entity.nbt,SETTINGS[setting],value);entity.data[22+setting]=value;}
}
