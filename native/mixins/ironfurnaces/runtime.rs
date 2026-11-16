// Apache-2.0; see LICENSE and NOTICE. Pinned Horizon adapter.
use super::{Game, iron_furnaces_rules as rules, iron_furnaces_energy_rules as power_rules,
    ironfurnaces_config as config, ironfurnaces_power as power, ironfurnaces_automation as automation,
    ironfurnaces_ui as ui, ironfurnaces_cooking as cooking};
use horizon_client::{containers::{BlockEntity, ModBlockEntity, ModContainer}, items::ItemStack};
use horizon_world::nbt::{Compound, JavaString, Tag};
use rules::{EfficiencyAugment, RecipeMode, Tier};
use power_rules::MachineMode;
pub const ID: &str = "ironfurnaces_native";
pub const HEATER: &str = "ironfurnaces:heater";
pub const SETTINGS: [&str;12] = ["Side0","Side1","Side2","Side3","Side4","Side5",
    "AutoInput","AutoOutput","RedstoneMode","RedstoneValue","AugmentTab","AutoSplit"];
pub fn owns(block:&str)->bool {block==HEATER||Tier::from_block(block).is_some()}
pub(super) fn nbt_key(name:&str)->JavaString {name.into()}
pub(super) fn number(nbt:&Compound,name:&str)->i32 {match nbt.get(&nbt_key(name)){Some(Tag::Int(v))=>*v,Some(Tag::Short(v))=>i32::from(*v),_=>0}}
pub(super) fn put(nbt:&mut Compound,key:&str,value:i32){nbt.insert(key.into(),Tag::Int(value));}
pub(super) fn string(nbt:&Compound,name:&str)->Option<String>{match nbt.get(&nbt_key(name)){Some(Tag::String(v))=>Some(v.to_string_lossy()),_=>None}}
pub(super) fn mode_from(e:&ModBlockEntity)->RecipeMode {e.items.get(3).and_then(Option::as_ref).and_then(|s|RecipeMode::from_item(&s.item)).unwrap_or_default()}
pub(super) fn efficiency_from(e:&ModBlockEntity)->EfficiencyAugment {e.items.get(4).and_then(Option::as_ref).and_then(|s|EfficiencyAugment::from_item(&s.item)).unwrap_or_default()}
pub(super) fn machine_mode(e:&ModBlockEntity)->MachineMode {e.items.get(5).and_then(Option::as_ref).and_then(|s|MachineMode::from_item(&s.item)).unwrap_or_default()}
pub(super) fn energy(e:&ModBlockEntity)->i32 {number(&e.nbt,"energy").max(0)}
pub(super) fn side(e:&ModBlockEntity,i:usize)->i32 {automation::side(e,i)}
/// Preserve v0.1 contents and hopper access; fresh placements start at Version=2.
pub fn initialize(e:&mut ModBlockEntity){
    if !owns(&e.id){return;}let size=e.items.len().max(if e.id==HEATER{1}else{19});e.items.resize(size,None);
    if number(&e.nbt,"Version")<2 {
        if e.id!=HEATER {
            for(field,slot)in[("Mode",3),("Efficiency",4)]{if let Some(Tag::String(id))=e.nbt.remove(&nbt_key(field)){if e.items[slot].is_none(){e.items[slot]=Some(ItemStack::new(&id.to_string_lossy(),1));}}}
            for(i,v)in[2,1,4,4,4,4].into_iter().enumerate(){if !e.nbt.contains_key(&nbt_key(SETTINGS[i])){put(&mut e.nbt,SETTINGS[i],v);}}
        }put(&mut e.nbt,"Version",2);
    }e.data.resize(64,0);
}
pub(super) fn effective_stack_max(game:&Game<'_>,stack:&ItemStack)->u32{
    let d=stack.data();if d.is_some_and(|d|d.other("!minecraft:max_stack_size").is_some()){return 1;}
    if let Some(Tag::Int(n))=d.and_then(|d|d.other("minecraft:max_stack_size"))&&(1..=99).contains(n){return(*n as u32).min(64);}
    game.items().get(&stack.item).map_or(64,|i|i.max_stack).clamp(1,64)
}
pub(super) fn transfer(s:&mut ItemStack,d:&mut Option<ItemStack>,max:u32){if s.count==0{return;}match d{
    Some(e)if e.same_item_same_components(s)=>{let n=max.saturating_sub(e.count).min(s.count);e.count+=n;s.count-=n;},
    None=>{let mut n=s.clone();n.count=n.count.min(max);s.count-=n.count;if n.count>0{*d=Some(n);}},_=>{}}}
fn augment_slot(item:&str)->Option<usize>{match power_rules::augment_channel(item){Some(power_rules::AugmentChannel::Recipe)=>Some(3),Some(power_rules::AugmentChannel::Efficiency)=>Some(4),Some(power_rules::AugmentChannel::Machine)=>Some(5),None=>None}}
pub fn quick_move(game:&mut Game<'_>,e:&mut ModBlockEntity,container_slots:usize,slot:usize,stack:&mut ItemStack,handled:&mut bool){
    if *handled||!game.native_on(ID)||!owns(&e.id){return;}initialize(e);let max=effective_stack_max(game,stack);
    if slot>=container_slots{
        let targets:Vec<usize>=if e.id==HEATER{vec![0]}else if let Some(slot)=augment_slot(&stack.item){vec![slot]}else{match machine_mode(e){
            MachineMode::Furnace=>vec![0,1],MachineMode::Generator=>vec![6],MachineMode::Factory=>config::active_factory_lanes(Tier::from_block(&e.id).unwrap()).map(|i|7+i).collect()}};
        for empty in[false,true]{for &target in &targets{if e.items[target].is_none()!=empty||!accepts(game,e,target,stack){continue;}
            let limit=if(3..=5).contains(&target)||matches!(&*stack.item,"ironfurnaces:item_heater"|"minecraft:bucket"){1}else{max};transfer(stack,&mut e.items[target],limit);}}
    }else{for empty in[false,true]{for d in game.inventory.iter_mut().take(36){if d.is_none()==empty{transfer(stack,d,max);}if stack.count==0{break;}}}}
    *handled=true;
}
pub fn screen(block:&str)->Option<horizon_client::mod_api::NativeScreen>{owns(block).then(||ui::layout(block,&[]))}
pub fn draw(game:&Game<'_>,block:&str,data:&[i32],batch:&mut horizon_client::hud::HudBatch,origin:[f32;2]){if owns(block){ui::draw(game,block,data,batch,origin);}}
pub fn packet_before(game:&mut Game<'_>,host:&mut super::lan::LanHost,conn:u32,packet:&horizon_protocol::packets::play::Serverbound){
    if !game.native_on(ID){return;}let horizon_protocol::packets::play::Serverbound::ContainerClick(click)=packet else{return;};
    let Some(guest)=host.hub.guests.iter_mut().find(|g|g.conn==conn)else{return;};if guest.spectator()||guest.dead{return;}
    let Some(menu)=guest.menu.as_mut().filter(|m|click.container!=0&&m.window==click.container)else{return;};
    let horizon_client::lan::menu::Slots::Block(pos)=menu.slots else{return;};
    let Some(BlockEntity::Mod(m))=game.world.block_entities.get(&pos)else{return;};if !owns(&m.id){return;}
    let hidden=ui::hidden(&m.id,&m.data,m.items.len());if hidden!=menu.hidden{menu.hidden=hidden;if let Some(play)=host.play.get_mut(&conn){play.crafting.machine=None;}}
    let fuels=game.items().items.values().filter(|i|game.res.providers().burn_time(game.items(),&ItemStack::new(&i.id,1),false)>0)
        .map(|i|i.id.to_string()).chain(["ironfurnaces:rainbow_coal".to_owned(),"ironfurnaces:item_heater".to_owned()]).collect();
    let recipe_items=game.items().items.values().filter(|i|game.recipes().cook(mode_from(m).recipe_type(),&ItemStack::new(&i.id,1)).is_some()).map(|i|i.id.to_string()).collect();
    let generator_fuels=game.items().items.values().filter(|i|power::accepts_generator(game,m,&ItemStack::new(&i.id,1))).map(|i|i.id.to_string()).collect();
    let active=Tier::from_block(&m.id).map_or(0..0,config::active_factory_lanes);
    horizon_client::menu::ironfurnaces_slots::set_context(machine_mode(m)as i32,active,recipe_items,fuels,generator_fuels);
    if click.slot==2||(13..19).contains(&click.slot)||click.input==6{game.native_with_mod_block_entity([pos.0,pos.1,pos.2],|m|{let n=output_count(m);let old=number(&m.nbt,"PendingExtraction");put(&mut m.nbt,"PendingExtraction",old.max(n));});}
}
fn output_count(e:&ModBlockEntity)->i32{[2,13,14,15,16,17,18].into_iter().filter_map(|i|e.items.get(i).and_then(Option::as_ref)).map(|s|s.count.min(i32::MAX as u32)as i32).sum()}
pub fn container(block:&str)->Option<ModContainer>{
    if block==HEATER{return Some(ModContainer{slots:1,faces:std::array::from_fn(|_|vec![0]),inputs:vec![0],outputs:vec![0],menu:Some("ironfurnaces:heater".into())});}Tier::from_block(block)?;
    Some(ModContainer{slots:19,faces:std::array::from_fn(|_|(0..19).collect()),inputs:vec![0,1,3,4,5,6,7,8,9,10,11,12],outputs:vec![1,2,6,13,14,15,16,17,18],menu:Some("ironfurnaces:machine".into())})
}
pub fn accepts(game:&Game<'_>,e:&ModBlockEntity,slot:usize,stack:&ItemStack)->bool{
    if e.id==HEATER{return slot==0&&&*stack.item=="ironfurnaces:item_heater";}let Some(tier)=Tier::from_block(&e.id)else{return true;};
    if(3..=5).contains(&slot){return augment_slot(&stack.item)==Some(slot);}match machine_mode(e){
        MachineMode::Furnace=>match slot{0=>game.recipes().cook(mode_from(e).recipe_type(),stack).is_some(),1=>game.res.providers().burn_time(game.items(),stack,false)>0||matches!(&*stack.item,"ironfurnaces:item_heater"|"ironfurnaces:rainbow_coal")||(&*stack.item=="minecraft:bucket"&&e.items.get(1).and_then(Option::as_ref).is_none()),_=>false},
        MachineMode::Factory=>(7..13).contains(&slot)&&config::active_factory_lanes(tier).contains(&(slot-7))&&game.recipes().cook(mode_from(e).recipe_type(),stack).is_some(),
        MachineMode::Generator=>slot==6&&power::accepts_generator(game,e,stack)
    }
}
pub(super) fn consume_fuel(game:&mut Game<'_>,pos:[i32;3],e:&mut ModBlockEntity,slot:usize)->bool{
    let Some(fuel)=e.items.get(slot).cloned().flatten()else{return false;};
    if &*fuel.item=="ironfurnaces:rainbow_coal"{
        let used=fuel.damage.saturating_add(1);
        if used<power_rules::RAINBOW_COAL_USES{if let Some(fuel)=e.items[slot].as_mut(){fuel.damage=used;}return true;}}
    let remainder=game.items().get(&fuel.item).and_then(|i|i.crafting_remainder.as_deref()).map(|id|ItemStack::new(id,1));
    if fuel.count<=1{e.items[slot]=remainder;}else{e.items[slot].as_mut().unwrap().count-=1;if let Some(stack)=remainder{game.campfire_drop((pos[0],pos[1],pos[2]),stack);}}true
}
pub fn interact(game:&mut Game<'_>,pos:[i32;3],canceled:&mut bool){
    if *canceled||!game.native_on(ID){return;}let key=(pos[0],pos[1],pos[2]);
    let state=game.world.block(pos[0],pos[1],pos[2]);let Some(state_data)=game.pack.assets.registry.state(state).cloned()else{return;};
    let held=game.held().cloned();
    if let Some(held)=&held{if let Some(target)=rules::upgrade_target(&held.item,&state_data.name){
        let mut saved=match game.world.block_entities.get(&key){
            Some(BlockEntity::Mod(m))if Tier::from_block(&m.id).is_some()=>(**m).clone(),
            Some(BlockEntity::Furnace(f))if state_data.name=="minecraft:furnace"=>{let mut m=ModBlockEntity::default();m.items=f.items.to_vec();
                for(k,v)in[("BurnTime",f.lit_remaining),("BurnTotal",f.lit_total),("CookTime",f.cooking_timer),("CookTotal",f.cooking_total)]{put(&mut m.nbt,k,v);}
                let mut used=Compound::new();for(id,count)in &f.recipes_used{put(&mut used,id,(*count).min(i32::MAX as u32)as i32);}m.nbt.insert("RecipesUsed".into(),Tag::Compound(used));m},_=>return};
        let source=Tier::from_block(&state_data.name);saved.id=target.block_id().into();initialize(&mut saved);
        let new_owner=string(&saved.nbt,"Owner").is_none().then(||game.native_acting_id());
        if let Some(owner)=new_owner{saved.nbt.insert("Owner".into(),Tag::String(owner.to_string().as_str().into()));}
        // Destination defaults also supply type/jovial/rainbow when upgrading vanilla.
        let Some(next)=game.pack.assets.registry.default_state(target.block_id())else{return;};
        let old_name=game.world.block_names.get(&key).cloned();let old_lock=game.world.block_lock(key).cloned();
        let old=game.world.block_entities.remove(&key);let dimension=game.dim.id().to_owned();
        if game.native_set_block(&dimension,pos,next,3){
            let augment=efficiency_from(&saved);let recipe=saved.items[0].as_ref().and_then(|s|game.recipes().cook(mode_from(&saved).recipe_type(),s));
            let old_total=if number(&saved.nbt,"CookTotal")>0{number(&saved.nbt,"CookTotal")}else{source.map_or(200,|t|config::cook_ticks(t,200,augment))as i32};
            let new_total=config::cook_ticks(target,recipe.map_or(200,|r|r.time),augment)as i32;
            if source.is_none(){
                if let Some(recipe)=recipe{saved.nbt.insert("Recipe".into(),Tag::String(recipe.id.as_str().into()));}
                if let Some(mut input)=saved.items[0].clone(){input.count=1;saved.nbt.insert("Input".into(),horizon_client::save::encode_items(&[Some(input)]));}
            }
            for field in["BurnTime","BurnTotal","CookTime"]{let n=number(&saved.nbt,field).max(0);put(&mut saved.nbt,field,(i64::from(n)*i64::from(new_total)/i64::from(old_total.max(1))).min(i64::from(i32::MAX))as i32);}put(&mut saved.nbt,"CookTotal",new_total);
            for lane in 0..6{let pk=format!("LaneProgress{lane}");let tk=format!("LaneTotal{lane}");let ot=number(&saved.nbt,&tk).max(1);
                let recipe=saved.items[7+lane].as_ref().and_then(|s|game.recipes().cook(mode_from(&saved).recipe_type(),s));let nt=config::cook_ticks(target,recipe.map_or(200,|r|r.time),augment)as i32;
                let p=number(&saved.nbt,&pk).max(0);put(&mut saved.nbt,&pk,(i64::from(p)*i64::from(nt)/i64::from(ot)).min(i64::from(nt-1))as i32);put(&mut saved.nbt,&tk,nt);
            }
            refresh_data(&mut saved);game.world.block_entities.insert(key,BlockEntity::Mod(Box::new(saved)));
            if let Some(name)=old_name{game.world.block_names.insert(key,name);}if let Some(lock)=old_lock{let mut tag=Compound::new();tag.insert("lock".into(),lock);game.world.set_block_lock(key,&Tag::Compound(tag));}
            for property in["facing","lit"]{if let Some(value)=state_data.properties.get(property){game.native_set_block_property(pos,property,value);}}
            if let Some(owner)=new_owner{power::track_furnace(game,pos,owner);}
            game.consume_held();game.native_block_entity_sync(pos);*canceled=true;
        }else if let Some(old)=old{game.world.block_entities.insert(key,old);}return;
    }}
    if Tier::from_block(&state_data.name).is_none(){return;}let sneak=game.keys.held(super::Mapping::Sneak);
    if let Some(held)=held{
        if &*held.item=="ironfurnaces:item_copy"{if automation::setting_copy(game,pos,&held,sneak){*canceled=true;}return;}
        if let Some(skin)=match &*held.item{"ironfurnaces:item_spooky"=>Some(1),"ironfurnaces:item_xmas"=>Some(2),_=>None}{
            game.native_with_mod_block_entity(pos,|m|put(&mut m.nbt,"Skin",skin));game.native_set_block_property(pos,"jovial",&skin.to_string());game.native_block_entity_sync(pos);*canceled=true;return;}
        if let Some(slot)=augment_slot(&held.item){let changed=game.native_with_mod_block_entity(pos,|m|{initialize(m);
            if m.items[slot].as_ref().is_some_and(|s|s.same_item_same_components(&held)){return None;}let mut installed=held.clone();installed.count=1;Some(m.items[slot].replace(installed))}).flatten();
            if let Some(previous)=changed{game.consume_held();if let Some(stack)=previous{game.campfire_drop(key,stack);}game.native_block_entity_sync(pos);*canceled=true;}}
    }else if sneak&&game.stack_in(game.selected).is_none()&&game.offhand.is_none(){game.native_with_mod_block_entity(pos,|m|put(&mut m.nbt,"Skin",0));game.native_set_block_property(pos,"jovial","0");game.native_block_entity_sync(pos);*canceled=true;}
}
pub fn before_remove(level:&mut super::GameLevel<'_>,pos:horizon_core::position::BlockPos,side_effects:bool){
    if !side_effects{return;}let Some(BlockEntity::Mod(e))=level.world.block_entities.get_mut(&(pos.x,pos.y,pos.z))else{return;};if !owns(&e.id){return;}initialize(e);
    if let Some(Tag::Compound(used))=e.nbt.remove(&nbt_key("RecipesUsed")){for(id,value)in used{let count=match value{Tag::Int(n)=>n.max(0)as u32,_=>0};
        let xp=level.recipes.cooking_experience(&id.to_string_lossy());let points=horizon_client::containers::create_experience(count,xp,&mut level.world.random);
        level.award_orbs([f64::from(pos.x)+0.5,f64::from(pos.y)+0.5,f64::from(pos.z)+0.5],points);}}
}
fn update_augments(game:&mut Game<'_>,pos:[i32;3],e:&mut ModBlockEntity){
    let signatures:[String;3]=std::array::from_fn(|i|e.items[3+i].as_ref().map_or(String::new(),|s|s.item.to_string()));
    let initialized=e.nbt.contains_key(&nbt_key("AugmentMachine"));let old=["AugmentRecipe","AugmentEfficiency","AugmentMachine"].map(|k|string(&e.nbt,k).unwrap_or_default());
    if initialized&&signatures!=old{
        if signatures[2]!=old[2]||(machine_mode(e)==MachineMode::Generator&&signatures[0]!=old[0]){
            for i in[0,1,2,6,7,8,9,10,11,12,13,14,15,16,17,18]{if let Some(stack)=e.items[i].take(){game.campfire_drop((pos[0],pos[1],pos[2]),stack);}}}
        for k in["CookTime","BurnTime","BurnTotal","GeneratorRemaining","GeneratorTotal"]{put(&mut e.nbt,k,0);}
        for i in 0..6{put(&mut e.nbt,&format!("LaneProgress{i}"),0);put(&mut e.nbt,&format!("LaneReserved{i}"),0);}
    }
    for(k,v)in["AugmentRecipe","AugmentEfficiency","AugmentMachine"].into_iter().zip(signatures){e.nbt.insert(k.into(),Tag::String(v.as_str().into()));}
}
pub fn tick(game:&mut Game<'_>,pos:[i32;3],e:&mut ModBlockEntity){
    if !game.native_on(ID)||!owns(&e.id){return;}initialize(e);if e.id==HEATER{power::tick_heater(game,pos,e);refresh_data(e);return;}
    let tier=Tier::from_block(&e.id).unwrap();let pending=number(&e.nbt,"PendingExtraction");e.nbt.remove(&nbt_key("PendingExtraction"));
    if pending>output_count(e){if let Some(Tag::Compound(used))=e.nbt.remove(&nbt_key("RecipesUsed")){for(id,value)in used{
        let count=match value{Tag::Int(n)=>n.max(0)as u32,_=>0};let xp=game.recipes().cooking_experience(&id.to_string_lossy());
        let points=horizon_client::containers::create_experience(count,xp,&mut game.world.random);game.spawn_orbs(pos.map(|v|f64::from(v)+0.5),points);}}}
    update_augments(game,pos,e);let run=automation::allows_run(game,pos,e);if run{automation::tick_io(game,pos,e);}
    match machine_mode(e){MachineMode::Furnace=>cooking::tick_furnace(game,pos,e,tier,run),MachineMode::Factory=>cooking::tick_factory(game,pos,e,tier,run),
        MachineMode::Generator=>if run{power::tick_generator(game,pos,e,tier);}else{put(&mut e.nbt,"GenerationRate",0);put(&mut e.nbt,"RainbowGen",0);}}
    refresh_data(e);let active=match machine_mode(e){MachineMode::Furnace=>number(&e.nbt,"BurnTime")>0,MachineMode::Generator=>number(&e.nbt,"GenerationRate")>0,MachineMode::Factory=>run&&number(&e.nbt,"FactoryActive")>0};
    let recipe_type=match mode_from(e){RecipeMode::Smelting=>0,RecipeMode::Smoking=>1,RecipeMode::Blasting=>2};
    for(k,v)in[("lit",if active{"true"}else{"false"}.to_owned()),("type",recipe_type.to_string()),("jovial",number(&e.nbt,"Skin").clamp(0,2).to_string())]{
        if game.native_block_property(pos,k).as_deref()!=Some(v.as_str()){game.native_set_block_property(pos,k,&v);}}
    if tier==Tier::Rainbow{let v=if number(&e.nbt,"RainbowGen")>0{"true"}else{"false"};if game.native_block_property(pos,"rainbow").as_deref()!=Some(v){game.native_set_block_property(pos,"rainbow",v);}}
}
pub(super) fn display_pair(current:i32,total:i32)->[i32;2]{if total<=0{return[0,1];}let shown=total.min(i32::from(i16::MAX));[(i64::from(current.clamp(0,total))*i64::from(shown)/i64::from(total))as i32,shown]}
pub fn refresh_data(e:&mut ModBlockEntity){
    e.data.resize(64,0);let heater=e.id==HEATER;let tier=Tier::from_block(&e.id);let generator=!heater&&machine_mode(e)==MachineMode::Generator;
    let burn=display_pair(number(&e.nbt,if generator{"GeneratorRemaining"}else{"BurnTime"}),number(&e.nbt,if generator{"GeneratorTotal"}else{"BurnTotal"}));
    let cook=display_pair(number(&e.nbt,"CookTime"),number(&e.nbt,"CookTotal"));e.data[0..2].copy_from_slice(&burn);e.data[2..4].copy_from_slice(&cook);
    e.data[4]=machine_mode(e)as i32;e.data[5]=tier.map_or(0,|t|config::storage_tier(t)as i32);let cap=tier.map_or(config::heater_capacity(),config::energy_capacity);
    for(i,n)in[(6,energy(e)as u32),(8,cap)]{e.data[i]=(n&65535)as i32;e.data[i+1]=(n>>16)as i32;}
    for i in 0..6{let v=display_pair(number(&e.nbt,&format!("LaneProgress{i}")),number(&e.nbt,&format!("LaneTotal{i}")));e.data[10+i*2..12+i*2].copy_from_slice(&v);}
    for(i,k)in SETTINGS.iter().enumerate(){e.data[22+i]=number(&e.nbt,k);}e.data[34]=match mode_from(e){RecipeMode::Smelting=>0,RecipeMode::Smoking=>1,RecipeMode::Blasting=>2};
    e.data[35]=match efficiency_from(e){EfficiencyAugment::None=>0,EfficiencyAugment::Speed=>1,EfficiencyAugment::Fuel=>2};let rate=number(&e.nbt,"GenerationRate").max(0)as u32;
    e.data[36]=(rate&65535)as i32;e.data[40]=(rate>>16)as i32;e.data[37]=number(&e.nbt,"RainbowGen");e.data[38]=i32::from(heater);e.data[39]=number(&e.nbt,"Skin");
}
