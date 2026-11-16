// Apache-2.0; see LICENSE and NOTICE.
use super::{Game,ironfurnaces as runtime,ironfurnaces_config as config,ironfurnaces_power as power,
    iron_furnaces_rules as rules,iron_furnaces_energy_rules as energy};
use runtime::{number,put,string,nbt_key,mode_from,efficiency_from,effective_stack_max};
use horizon_client::{containers::ModBlockEntity,items::ItemStack};
use horizon_world::nbt::{Compound,Tag};
use rules::Tier;

fn recipe_matches(e:&mut ModBlockEntity,input:usize,key:&str,id:&str)->bool{
    let tag=e.items[input].as_ref().map(|s|{let mut s=s.clone();s.count=1;horizon_client::save::encode_items(&[Some(s)])});
    let input_key=format!("{key}Input");let recipe_key=format!("{key}Recipe");
    let same=e.nbt.get(&nbt_key(&input_key))==tag.as_ref()&&string(&e.nbt,&recipe_key).as_deref().unwrap_or("")==id;
    if let Some(tag)=tag{e.nbt.insert(input_key.as_str().into(),tag);}else{e.nbt.remove(&nbt_key(&input_key));}
    e.nbt.insert(recipe_key.as_str().into(),Tag::String(id.into()));same
}
fn count(game:&Game<'_>,e:&ModBlockEntity,input:usize,output:usize,tier:Tier,result:&ItemStack)->u32{
    if !e.items[output].as_ref().is_none_or(|s|s.same_item_same_components(result)){return 0;}
    rules::batch_count(config::batch_limit(tier),e.items[input].as_ref().map_or(0,|s|s.count),e.items[output].as_ref().map_or(0,|s|s.count),result.count,effective_stack_max(game,result))
}
fn finish(game:&mut Game<'_>,pos:[i32;3],e:&mut ModBlockEntity,input:usize,output:usize,result:&ItemStack,id:&str,count:u32){
    let input_id=e.items[input].as_ref().map(|s|s.item.to_string()).unwrap_or_default();
    if let Some(s)=e.items[output].as_mut(){s.count+=result.count*count;}else{let mut s=result.clone();s.count*=count;e.items[output]=Some(s);}
    if let Some(s)=e.items[input].as_mut(){s.count-=count;}if e.items[input].as_ref().is_some_and(|s|s.count==0){e.items[input]=None;}
    if input==0&&input_id=="minecraft:wet_sponge"&&e.items[1].as_ref().is_some_and(|s|&*s.item=="minecraft:bucket"){e.items[1]=Some(ItemStack::new("minecraft:water_bucket",1));}
    let mut used=match e.nbt.remove(&nbt_key("RecipesUsed")){Some(Tag::Compound(v))=>v,_=>Compound::new()};let old=number(&used,id).max(0);
    put(&mut used,id,old.saturating_add(count as i32));e.nbt.insert("RecipesUsed".into(),Tag::Compound(used));game.native_furnace_smelt(pos,&input_id,&result.item);
}
pub fn tick_furnace(game:&mut Game<'_>,pos:[i32;3],e:&mut ModBlockEntity,tier:Tier,run:bool){
    if !run{return;}let augment=efficiency_from(e);let mut burn=number(&e.nbt,"BurnTime").saturating_sub(1).max(0);
    let mut total_burn=number(&e.nbt,"BurnTotal").max(0);let mut progress=number(&e.nbt,"CookTime").max(0);
    let recipe=e.items[0].as_ref().and_then(|s|game.recipes().cook(mode_from(e).recipe_type(),s)).cloned();
    let total=config::cook_ticks(tier,recipe.as_ref().map_or(200,|r|r.time),augment)as i32;
    // The empty prefix deliberately retains the v0.1 Input/Recipe keys.
    if !recipe_matches(e,0,"",recipe.as_ref().map_or("",|r|r.id.as_str())){progress=0;}
    if let Some(recipe)=recipe{let n=count(game,e,0,2,tier,&recipe.result);if n>0{
        if burn==0{
            if let Some((ticks,total))=power::wireless_ignite(game,pos,e,total as u32,augment){burn=ticks as i32;total_burn=total as i32;}
            else if let Some(fuel)=e.items[1].clone(){
                let vanilla=if &*fuel.item=="ironfurnaces:rainbow_coal"&&fuel.damage<energy::RAINBOW_COAL_USES{energy::RAINBOW_COAL_FUEL_TICKS}else{game.res.providers().burn_time(game.items(),&fuel,false).max(0)as u32};
                let ticks=rules::fuel_ticks(vanilla,total as u32,augment)as i32;
                if ticks>0&&runtime::consume_fuel(game,pos,e,1){burn=ticks;total_burn=ticks;}
            }
        }
        if burn>0{progress=progress.saturating_add(1);if progress>=total{progress=0;finish(game,pos,e,0,2,&recipe.result,&recipe.id,n);}}
        else{progress=(progress-2).max(0);}
    }else{progress=0;}}else{progress=0;}
    for(k,v)in[("BurnTime",burn),("BurnTotal",total_burn),("CookTime",progress),("CookTotal",total)]{put(&mut e.nbt,k,v);}
}
fn split_inputs(game:&Game<'_>,e:&mut ModBlockEntity,tier:Tier){
    let lanes:Vec<usize>=config::active_factory_lanes(tier).map(|i|7+i).collect();
    let mut done=std::collections::HashSet::new();
    for &slot in &lanes{
        if done.contains(&slot){continue;}let Some(sample)=e.items[slot].clone()else{continue;};
        let targets:Vec<usize>=lanes.iter().copied().filter(|i|!done.contains(i)&&e.items[*i].as_ref().is_none_or(|s|s.same_item_same_components(&sample))).collect();
        let total:u32=targets.iter().map(|i|e.items[*i].as_ref().map_or(0,|s|s.count)).sum();
        let max=effective_stack_max(game,&sample);let each=(total/targets.len()as u32).min(max);let mut extra=total-each*targets.len()as u32;
        for i in targets{let n=(each+u32::from(extra>0)).min(max);extra=extra.saturating_sub(n-each);let mut stack=sample.clone();stack.count=n;e.items[i]=(n>0).then_some(stack);done.insert(i);}
    }
}
pub fn tick_factory(game:&mut Game<'_>,pos:[i32;3],e:&mut ModBlockEntity,tier:Tier,run:bool){
    put(&mut e.nbt,"FactoryActive",0);if !run{return;}
    if number(&e.nbt,"AutoSplit")!=0{split_inputs(game,e,tier);}
    let augment=efficiency_from(e);let mut stored=runtime::energy(e).min(config::energy_capacity(tier)as i32)as u32;
    let active=config::active_factory_lanes(tier);
    // A configuration reload can reduce the lane count. Keep hidden items,
    // but release their unspent reservations so active lanes cannot deadlock.
    for lane in 0..6{if !active.contains(&lane){put(&mut e.nbt,&format!("LaneReserved{lane}"),0);put(&mut e.nbt,&format!("LaneProgress{lane}"),0);}}
    // Reserve each operation's complete cost before its first tick. Unspent
    // reservations remain inside stored energy, preventing parallel overdraw.
    let mut reserved:u32=(0..6).map(|i|number(&e.nbt,&format!("LaneReserved{i}")).max(0)as u32).fold(0,u32::saturating_add);
    for lane in active{
        let input=7+lane;let output=13+lane;let prefix=format!("Lane{lane}");
        let pk=format!("LaneProgress{lane}");let tk=format!("LaneTotal{lane}");let ck=format!("LaneCost{lane}");let rk=format!("LaneReserved{lane}");
        let mut progress=number(&e.nbt,&pk).max(0)as u32;let mut remaining=number(&e.nbt,&rk).max(0)as u32;
        let recipe=e.items[input].as_ref().and_then(|s|game.recipes().cook(mode_from(e).recipe_type(),s)).cloned();
        let total=config::cook_ticks(tier,recipe.as_ref().map_or(200,|r|r.time),augment);
        let same=recipe_matches(e,input,&prefix,recipe.as_ref().map_or("",|r|r.id.as_str()));
        let n=recipe.as_ref().map_or(0,|r|count(game,e,input,output,tier,&r.result));
        if !same||n==0||progress>=total{reserved=reserved.saturating_sub(remaining);remaining=0;progress=0;}
        if let Some(recipe)=recipe&&n>0{
            if progress==0&&remaining==0{let cost=energy::factory_recipe_energy(recipe.time,augment);
                if energy::factory_can_start(stored,reserved,cost){remaining=cost;reserved=reserved.saturating_add(cost);put(&mut e.nbt,&ck,cost as i32);}}
            if remaining>0{
                // Remainder on the final tick preserves exact operation costs;
                // remaining also survives tier upgrades and configuration edits.
                let debit=remaining/(total-progress);
                if stored>=debit{stored-=debit;remaining-=debit;reserved=reserved.saturating_sub(debit);progress+=1;put(&mut e.nbt,"FactoryActive",1);
                    if progress>=total{progress=0;remaining=0;finish(game,pos,e,input,output,&recipe.result,&recipe.id,n);}}
            }
        }
        put(&mut e.nbt,&pk,progress as i32);put(&mut e.nbt,&tk,total as i32);put(&mut e.nbt,&rk,remaining as i32);
    }put(&mut e.nbt,"energy",stored as i32);
}
