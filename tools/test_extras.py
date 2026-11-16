"""Actual heater break/place persistence and TNT-to-Rainbow-Coal regressions."""
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

from smoke import ROOT, WORLD, require
from test_power import PowerSession


def heater_drop_place(s):
    pos,target=(4,200,2),(6,200,2)
    s.setblock(pos,"ironfurnaces:heater")
    s.sprint(1)
    s.merge(pos,energy=543_210)
    s.replace(pos,0,"ironfurnaces:item_heater")
    s.sprint(1)
    s.held("minecraft:netherite_pickaxe")
    s.call("tp",4.5,200,.5)
    s.command("gamemode survival")
    try:
        # Mining progress requires actual server ticks; no receiver consumes
        # energy in this fixture, so the saved amount remains exact.
        s.command("tick unfreeze")
        s.call("break",*pos,"--timeout",20,timeout=25)
        s.command("tick freeze")
        require("minecraft:air" in s.call("block",*pos)["state"],"Heater was not mined")
        s.call("tp",4.5,200,2.5)
        s.call("wait",4)
        s.call("entities",12,"minecraft:item")
        # Block and inventory drops have different random lateral motion.
        # Gather the existing entities; do not create replacement items.
        s.command("tp @e[type=minecraft:item,distance=..12] @s")
        s.sprint(40)
        s.call("wait",4)
        inventory=s.call("inventory")
        require(inventory["counts"].get("ironfurnaces:heater",0)==1,("Heater block duplicated or lost",inventory))
        require(inventory["counts"].get("ironfurnaces:item_heater",0)==1,("Receiver duplicated or lost",inventory))
        stack=next((v for v in inventory["hotbar"] if v.get("item")=="ironfurnaces:heater"),None)
        require(stack is not None,("Heater pickup was not in hotbar",inventory))
        require("543210" in stack.get("components","") and "ironfurnaces:energy" in stack.get("components",""),
                ("Stored energy component missing",stack))
        s.call("select",stack["slot"]+1)
        s.call("tp",6.5,200,.5)
        s.call("place",*target)
        s.sprint(1)
        require(s.integer(target,"energy")==543_210,"Placed heater lost its energy")
        s.expect_slot(target,0,"ironfurnaces:item_heater",0)
        require(s.call("inventory")["counts"].get("ironfurnaces:heater",0)==0,"Placed heater was not consumed")
        return {"energy":543_210,"heater_drops":1,"receiver_drops":1,"placement":"restored energy"}
    finally:
        s.command("tick freeze")
        s.command("gamemode creative")


def rainbow_explosion(s):
    pos=(14,200,8)
    s.command("kill @e[type=minecraft:item]")
    s.machine(pos,"million")
    s.replace(pos,0,"minecraft:diamond",7)
    s.replace(pos,1,"minecraft:coal",3)
    s.call("tp",14.5,200,0.5)
    s.command("summon minecraft:tnt 14.5 200.2 8.5 {fuse:1s}")
    s.sprint(5)
    s.call("wait",4)
    state=s.call("block",*pos)["state"]
    require("minecraft:air" in state,("TNT did not destroy million furnace",state))
    items=s.call("entities",32,"minecraft:item")["entities"]
    drops=[entity["item"] for entity in items if entity.get("kind")=="item"]
    coal=sum(item.get("count",0) for item in drops if item.get("item")=="ironfurnaces:rainbow_coal")
    require(coal==1,("Explosion coal did not survive exactly once",drops))
    require(not any(item.get("item") in ("ironfurnaces:million_furnace","minecraft:diamond","minecraft:coal") for item in drops),
            ("Explosion conversion leaked furnace inventory or block",drops))
    s.sprint(20)
    after=s.call("entities",32,"minecraft:item")["entities"]
    require(sum(v.get("item",{}).get("count",0) for v in after
                if v.get("item",{}).get("item")=="ironfurnaces:rainbow_coal")==1,
            ("Coal disappeared after blast",after))
    return {"explosive":"minecraft:tnt","rainbow_coal":1,"original_inventory":"destroyed"}


def furnace_self_drop(s):
    pos=(10,200,2)
    s.machine(pos,"iron")
    s.replace(pos,0,"minecraft:raw_iron",2)
    s.held("minecraft:netherite_pickaxe")
    s.call("tp",10.5,200,.5)
    s.command("gamemode survival")
    s.call("wait",6)
    try:
        s.command("tick unfreeze")
        broken=s.call("break",*pos,"--timeout",20,timeout=25)
        s.command("tick freeze")
        s.call("tp",10.5,200,2.5)
        s.command("tp @e[type=minecraft:item,distance=..8] @s")
        s.sprint(40);s.call("wait",4)
        inventory=s.call("inventory")
        require(inventory["counts"].get("ironfurnaces:iron_furnace",0)==1,("Furnace self drop lost or duplicated",broken,inventory))
        require(inventory["counts"].get("minecraft:raw_iron",0)==2,("Furnace contents lost or duplicated",broken,inventory))
        return {"iron_furnace":1,"raw_iron_contents":2,"survival_tool":"netherite_pickaxe"}
    finally:
        s.command("tick freeze");s.command("gamemode creative")


def configuration(s):
    pos=(4,200,2)
    path=s.game_dir/"config/ironfurnaces.json"
    original=path.read_bytes()
    settings=json.loads(original)
    override=s.game_dir/"saves"/WORLD/"serverconfig/ironfurnaces.json"
    require(not override.exists(),"Disposable world unexpectedly has a config override")
    try:
        settings["furnaces"]["iron_furnace"]["speed"]=80
        path.write_text(json.dumps(settings),encoding="utf-8")
        s.machine(pos)
        deadline=time.monotonic()+15
        while True:
            s.call("wait",10)
            s.sprint(1)
            if s.integer(pos,"CookTotal")==80:break
            require(time.monotonic()<deadline,"Changed configuration was not reloaded")
            time.sleep(.1)
        s.seed(pos,"iron",1)
        s.sprint(79)
        s.expect_slot(pos,2,"minecraft:iron_ingot",0)
        s.sprint(1)
        s.expect_slot(pos,2,"minecraft:iron_ingot",1)
        path.write_text("{",encoding="utf-8")
        time.sleep(1.2)
        s.call("wait",10)
        s.sprint(1)
        require(s.integer(pos,"CookTotal")==80,"Invalid JSON discarded the last valid settings")
        path.write_bytes(original)
        settings["furnaces"]["iron_furnace"]["speed"]=60
        override.parent.mkdir(parents=True,exist_ok=True)
        override.write_text(json.dumps(settings),encoding="utf-8")
        s.call("save")
        s.stop()
        s.start()
        s.machine(pos)
        require(s.integer(pos,"CookTotal")==60,"World configuration did not override the global file")
        return {"live_reload_ticks":80,"invalid_json":"last valid settings retained",
                "world_override_ticks":60}
    finally:
        path.write_bytes(original)
        override.unlink(missing_ok=True)


CASES={"heater_drop_place":heater_drop_place,"rainbow_explosion":rainbow_explosion,
       "furnace_self_drop":furnace_self_drop,"configuration":configuration}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ("game","ctl","resources","run-dir"):
        parser.add_argument(f"--{name}",type=Path,required=True)
    parser.add_argument("--content",type=Path,default=ROOT/".build/migration-v3/mod")
    parser.add_argument("--visible",action="store_true")
    parser.add_argument("--only",default=",".join(CASES))
    args=parser.parse_args();args.windowed=True;args.offscreen=not args.visible
    for name in ("game","ctl","resources","content"):
        setattr(args,name,getattr(args,name).resolve(strict=True))
    if args.run_dir.exists():parser.error("--run-dir must be fresh")
    if args.run_dir.resolve().is_relative_to(args.resources):parser.error("run directory must be outside resources")
    cases=args.only.split(",")
    if not all(name in CASES for name in cases):parser.error("unknown --only case")
    s=PowerSession(args)
    try:
        s.start()
        for name in cases:
            s.call("close")
            s.run_case(name,CASES[name])
    finally:
        try:s.stop()
        finally:s.write()
    print(f"Extras passed: {s.run/'results.json'}",flush=True)


if __name__=="__main__":main()
