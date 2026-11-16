"""Exercise real machine menu clicks and capture the machine screens.

Uses an isolated rendered game and ordinary control-socket input. Assertions
cover server inventory/settings and timing; PNGs require visual inspection.
"""
from __future__ import annotations

import argparse
import math
import re
import uuid
from pathlib import Path

from smoke import ROOT, require
from test_power import PowerSession


def capture(s, name, pos):
    # Move away from controls through ordinary input so hover tooltips do not
    # cover the values we are capturing (there is no carried stack here).
    s.call("click-at",10,10)
    s.call("wait", 6)
    path = s.run / f"{name}.png"
    screen = s.call("screen")
    require(screen["screen"] in ("inventory","container"), (name, "machine menu not open", screen))
    s.call("screenshot", str(path))
    require(path.is_file() and path.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n", (name, "missing PNG"))
    captures = s.results.setdefault("captures", {"status": "recorded", "detail": {}})
    captures["detail"][name] = {"screenshot": str(path), "screen": screen,
                                 "menu": s.call("menu"), "slots": s.slots(pos)}
    s.write()


def click(s, x, y, right=False):
    # click-at sends the real window input path, including native screen
    # buttons. click-gui goes directly to inventory slots and skips them.
    width,height=s.call("ui")["gui_size"]
    origin=(math.floor((width-276)/2),math.floor((height-184)/2))
    s.call("click-at",origin[0]+x,origin[1]+y,*(["right"] if right else []))
    s.call("wait", 5)


def factory_screens(s):
    pos = (4, 200, 2)
    details = {}
    for tier, ticks, lanes in [("iron",160,range(2,4)), ("gold",120,range(1,5)),
                                ("diamond",80,range(6))]:
        s.call("close")
        s.machine(pos, tier, "factory")
        s.merge(pos, energy=40_000)
        for lane in lanes:
            s.replace(pos, 7+lane, "minecraft:raw_iron", 4)
        s.sprint(ticks//2)
        for lane in lanes:
            require(s.integer(pos, f"LaneProgress{lane}") == ticks//2,
                    (tier, lane, "wrong half-progress"))
            s.expect_slot(pos, 13+lane, "minecraft:iron_ingot", 0)
        s.interact(pos, close=False)
        capture(s, f"factory-{len(lanes)}-lanes-half", pos)
        details[tier] = {"lanes": len(lanes), "progress": ticks//2, "duration": ticks}
    return details


def generator_screen(s):
    pos = (4,200,2)
    s.call("close")
    s.machine(pos,"iron","generator")
    s.replace(pos,6,"minecraft:coal",2)
    s.sprint(80)
    require(s.integer(pos,"energy")==3200,"generator energy checkpoint")
    require(s.integer(pos,"GenerationRate")==40,"generator rate checkpoint")
    s.interact(pos,close=False)
    capture(s,"generator-active",pos)
    return {"energy":3200,"rate":40}


def interactive_controls(s):
    pos = (4,200,2)
    s.call("close")
    s.machine(pos)
    s.interact(pos,close=False)
    click(s,166,9)
    require(s.integer(pos,"AugmentTab")==1,"augment page button did not reach server")
    s.command("item replace entity @s hotbar.0 with ironfurnaces:augment_speed 2")
    s.call("wait",4)
    s.call("click-slot",0)
    s.call("wait",4)
    s.call("click-slot",304)
    s.call("wait",4)
    s.expect_slot(pos,4,"ironfurnaces:augment_speed",1)
    require(s.call("menu")["carried"]["count"]==1,"augment slot took more than one")
    s.call("click-slot",304)
    s.call("wait",4)
    s.expect_slot(pos,4,"ironfurnaces:augment_speed",1)
    require(s.call("menu")["carried"]["count"]==1,"full augment slot accepted a second item")
    s.call("click-slot",0)
    s.call("wait",4)
    for inventory_slot, machine_slot, augment in [(1,303,"blasting"),(2,305,"factory")]:
        s.command(f"item replace entity @s hotbar.{inventory_slot} with ironfurnaces:augment_{augment} 1")
        s.call("wait",4)
        s.call("click-slot",inventory_slot)
        s.call("wait",4)
        s.call("click-slot",machine_slot)
        s.call("wait",4)
        s.expect_slot(pos,machine_slot-300,f"ironfurnaces:augment_{augment}",1)
    s.sprint(1)
    capture(s,"augments-installed",pos)
    click(s,166,9)
    require(s.integer(pos,"AugmentTab")==0,"augment page did not close")
    click(s,198,29)
    require(s.integer(pos,"Side0")==3,"side forward click failed")
    click(s,198,29,True)
    require(s.integer(pos,"Side0")==2,"side reverse click failed")
    click(s,205,67)
    click(s,251,67)
    click(s,228,154)
    for field in ("AutoInput","AutoOutput","AutoSplit"):
        require(s.integer(pos,field)==1,(field,"toggle click failed"))
    click(s,16,63)
    require(s.integer(pos,"AutoSplit")==0,"factory S shortcut did not toggle auto split off")
    click(s,16,63)
    require(s.integer(pos,"AutoSplit")==1,"factory S shortcut did not toggle auto split on")
    click(s,264,97)  # Redstone comparator subtraction mode.
    require(s.integer(pos,"RedstoneMode")==4,"redstone mode click failed")
    click(s,263,116)
    require(s.integer(pos,"RedstoneValue")==1,"redstone + click failed")
    capture(s,"settings-configured",pos)
    click(s,228,136)
    require([s.integer(pos,f"Side{i}") for i in range(6)]==[0]*6,"reset sides failed")
    # Remove an augment through the actual slot and return it to inventory.
    click(s,166,9)
    s.call("click-slot",304)
    s.call("wait",4)
    s.expect_slot(pos,4,"ironfurnaces:augment_speed",0)
    require(s.call("menu")["carried"]["item"]=="ironfurnaces:augment_speed","augment removal lost item")
    s.call("click-slot",1)
    s.call("wait",4)
    require(s.call("menu")["carried"] is None,"removed augment did not return to inventory")
    return {"augment_slot_limit":1,"augment_channels":"installed via GUI",
            "augment_removal":"returned via GUI","side_cycle":"forward and reverse",
            "auto_input_output_split":True,"redstone_mode":4,"redstone_value":1,"reset_sides":True}


def heater_screen(s):
    pos=(4,200,2)
    s.call("close")
    s.setblock(pos,"minecraft:air")
    s.setblock(pos,"ironfurnaces:heater")
    s.sprint(1)
    s.replace(pos,0,"ironfurnaces:item_heater")
    s.merge(pos,energy=640_000)
    s.sprint(1)
    item_data=s.command("data get block 4 200 2 Items")
    require("HeaterPosX" in str(item_data), ("receiver not bound",item_data))
    s.interact(pos,close=False)
    capture(s,"heater-bound",pos)
    s.interact(pos,"minecraft:stick",sneak=True,close=False)
    s.call("wait",4)
    require(s.call("screen")["screen"]=="playing",
            "Sneaking with a main-hand item opened the heater through the empty offhand")
    # Check the related empty-main-hand skin-reset gate with a full offhand.
    skin_pos=(6,200,2)
    s.machine(skin_pos)
    s.interact(skin_pos,"ironfurnaces:item_spooky")
    s.held(None)
    s.command("item replace entity @s weapon.offhand with minecraft:stick 1")
    s.call("wait",4)
    s.use_held(skin_pos,sneak=True)
    require(s.integer(skin_pos,"Skin")==1,"Sneaking with an occupied offhand cleared the seasonal skin")
    return {"energy":640_000,"capacity":1_000_000,"receiver":"bound",
            "sneak_main_item_empty_offhand":"kept heater closed",
            "sneak_empty_main_occupied_offhand":"preserved skin"}


def coal_bars(s):
    pos=(4,200,2)
    s.call("close");s.machine(pos);s.interact(pos,close=False)
    for slot,damage in enumerate((0,2560,4608)):
        s.command(f"item replace entity @s hotbar.{slot} with ironfurnaces:rainbow_coal[minecraft:damage={damage}] 1")
    capture(s,"rainbow-coal-bars",pos)
    return {"remaining_uses":[5120,2560,512],"visual_review":"Full green, half yellow, low red-orange bars"}


def emerald_progress(s):
    from check_visual import visual_progress
    previous=dict(s.results.get("captures",{}).get("detail",{}))
    s.args.tier="emerald"
    detail=visual_progress(s)
    current=s.results["captures"]["detail"]
    s.results["captures"]["detail"]={**previous,**{f"emerald-{key}":value for key,value in current.items()}}
    s.write()
    return detail


def placed_owner(s):
    pos=(8,200,2)
    s.call("close");s.setblock(pos,"minecraft:air")
    expected=uuid.UUID(s.owner()).int
    s.held("ironfurnaces:iron_furnace")
    s.call("tp",8.5,200,.5);s.call("wait",6)
    s.call("place",*pos);s.sprint(1)
    reply=s.command("data get block 8 200 2 Owner")
    found=None
    for line in reply.get("chat",[]):
        match=re.search(r'has the following block data:\s*"([^"]+)"',line)
        if match:found=match.group(1)
    require(found is not None,("Actual block placement did not record Owner",reply))
    actual=int(found) if found.isdecimal() else uuid.UUID(found).int
    require(actual==expected,("Placement owner differs from acting player",actual,expected))
    return {"placement":"real held block use","owner_matches_player":True}


CASES={"factories":factory_screens,"generator":generator_screen,
       "controls":interactive_controls,"heater":heater_screen,"coal":coal_bars,
       "emerald":emerald_progress,"owner":placed_owner}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ("game","ctl","resources","run-dir"):
        parser.add_argument(f"--{name}",type=Path,required=True)
    parser.add_argument("--content",type=Path,default=ROOT/".build/migration-v3/mod")
    parser.add_argument("--visible",action="store_true")
    parser.add_argument("--only",default=",".join(CASES))
    args=parser.parse_args()
    args.windowed=True;args.offscreen=not args.visible
    for name in ("game","ctl","resources","content"):
        setattr(args,name,getattr(args,name).resolve(strict=True))
    if args.run_dir.exists():parser.error("--run-dir must be fresh")
    if args.run_dir.resolve().is_relative_to(args.resources):parser.error("run directory must be outside resources")
    cases=args.only.split(",")
    if not all(name in CASES for name in cases):parser.error("unknown --only case")
    s=PowerSession(args)
    try:
        s.start();s.call("options-set","chatVisibility","2")
        s.call("options-set","tutorialStep","none")
        for name in cases:s.run_case(name,CASES[name])
    finally:
        try:s.stop()
        finally:s.write()
    print(f"GUI assertions passed; inspect PNGs in {s.run}",flush=True)


if __name__=="__main__":main()
