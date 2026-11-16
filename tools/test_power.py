"""Exercise the full native port in a disposable, actual Horizon game world.

Uses the same isolated launcher/control harness as smoke.py. These are behavior
checks against upstream defaults, not tests that import the implementation.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import time
import uuid

from smoke import ROOT, Session, require


class PowerSession(Session):
    def machine(self, pos, tier="iron", mode=None, efficiency=None):
        self.setblock(pos, "minecraft:air")
        self.setblock(pos, f"ironfurnaces:{tier}_furnace")
        require(len(self.slots(pos)) == 19, ("v2 machine inventory", pos, self.slots(pos)))
        if mode:
            self.interact(pos, f"ironfurnaces:augment_{mode}")
            self.expect_slot(pos, 5, f"ironfurnaces:augment_{mode}", 1)
        if efficiency:
            self.interact(pos, f"ironfurnaces:augment_{efficiency}")
            self.expect_slot(pos, 4, f"ironfurnaces:augment_{efficiency}", 1)
        # Upstream reconciles changed augment channels on the next server tick,
        # ejecting the previous mode's working inventory. Seed inputs afterward.
        self.sprint(1)

    def merge(self, pos, **fields):
        self.command(f"data merge block {' '.join(map(str,pos))} " + json.dumps(fields))

    def replace(self, pos, slot, item, count=1):
        reply=self.command(f"item replace block {' '.join(map(str,pos))} container.{slot} with {item} {count}")
        require(any(line.startswith("Replaced ") for line in reply.get("chat",[])),
                ("Item fixture was not installed",pos,slot,item,reply))

    def wait_loaded(self, pos):
        # wait-ready can return before a teleport queues the next frame's
        # streaming work. Observe the destination chunk before editing it.
        deadline=time.monotonic()+60
        while time.monotonic()<deadline:
            self.call("wait",10)
            self.sprint(1)
            if self.call("block",*pos).get("loaded"):
                return
        raise TimeoutError(("Destination chunk did not load",pos))

    def use_held(self, pos, sneak=False):
        """Interact without clearing the copy tool/receiver's saved components."""
        self.call("close")
        self.call("tp", pos[0]+.5, pos[1], pos[2]-2)
        if sneak:
            self.call("pad", "rs", "down")
            self.call("wait", 5)
            require(self.call("state")["player"]["sneaking"], "Sneak did not engage")
        try:
            reply = self.call("interact", *pos, "north")
        finally:
            self.call("close")
            if sneak:
                self.call("pad", "rs", "up")
                self.call("wait", 3)
                if self.call("state")["player"]["sneaking"]:
                    self.call("pad", "rs", "tap")
                    self.call("wait", 3)
                self.call("pad", "release")
        return reply

    def count(self, pos, item):
        return sum(slot["count"] for slot in self.slots(pos) if slot and slot["item"] == item)

    def owner(self):
        reply = self.command("data get entity @s UUID")
        for line in reply.get("chat", []):
            found = re.search(r"\[I;\s*(-?\d+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)\]", line)
            if found:
                value = 0
                for part in found.groups():
                    value = (value << 32) | (int(part) & 0xffffffff)
                return str(uuid.UUID(int=value))
        raise AssertionError(("Could not read acting player's UUID", reply))


def migration(s):
    pos = (4, 200, 2)
    s.machine(pos)
    s.replace(pos, 0, "minecraft:raw_iron", 3)
    s.replace(pos, 1, "minecraft:coal", 2)
    for index in range(6):
        s.command(f"data remove block 4 200 2 Side{index}")
    s.merge(pos, Version=1, Mode="ironfurnaces:augment_blasting", Efficiency="ironfurnaces:augment_fuel",
            BurnTime=0, BurnTotal=0, CookTime=0)
    s.sprint(1)
    require(s.integer(pos, "Version") == 2, "Legacy save was not migrated")
    s.expect_slot(pos, 3, "ironfurnaces:augment_blasting", 1)
    s.expect_slot(pos, 4, "ironfurnaces:augment_fuel", 1)
    s.expect_slot(pos, 0, "minecraft:raw_iron", 3)
    require([s.integer(pos, f"Side{i}") for i in range(6)] == [2, 1, 4, 4, 4, 4], "Legacy routes lost")
    s.sprint(200)
    require(s.count(pos, "minecraft:iron_ingot") >= 1, "Migrated machine did not cook")
    return {"version": 2, "slots": 19, "legacy_augments_and_routes": "preserved"}


def vanilla_upgrade(s):
    pos=(4,200,2)
    s.setblock(pos,"minecraft:air")
    s.setblock(pos,"minecraft:furnace[facing=west]")
    s.replace(pos,0,"minecraft:raw_iron",3)
    s.replace(pos,1,"minecraft:coal",2)
    s.replace(pos,2,"minecraft:iron_ingot",2)
    s.merge(pos,CustomName="Upgrade checkpoint",lock={"items":"ironfurnaces:upgrade_iron"})
    # Establish paid fuel and partial progress through the vanilla engine.
    s.sprint(57)
    fields={"BurnTime":"lit_time_remaining","BurnTotal":"lit_total_time",
            "CookTime":"cooking_time_spent","CookTotal":"cooking_total_time"}
    before={new:s.integer(pos,old) for new,old in fields.items()}
    require(0<before["CookTime"]<before["CookTotal"],("Vanilla did not partially cook",before))
    require(before["BurnTime"]>0 and before["BurnTotal"]>0,"Vanilla did not ignite paid fuel")
    contents=s.slots(pos)
    def metadata(field):
        reply=s.command(f"data get block 4 200 2 {field}")
        for line in reply.get("chat",[]):
            if "has the following block data:" in line:
                return line.split("has the following block data:",1)[1].strip()
        raise AssertionError((field,"Missing block metadata",reply))
    named,locked=metadata("CustomName"),metadata("lock")
    require("Upgrade checkpoint" in named and "ironfurnaces:upgrade_iron" in locked,
            ("Name/lock fixture was not applied",named,locked))
    s.interact(pos,"ironfurnaces:upgrade_iron")
    require(s.call("block",*pos)["block"]=="ironfurnaces:iron_furnace","Vanilla upgrade did not apply")
    require("facing=west" in s.call("block",*pos)["state"],"Vanilla upgrade lost facing")
    require(s.slots(pos)[:3]==contents,"Vanilla upgrade changed input, fuel, or output")
    total=s.integer(pos,"CookTotal")
    require(total==160,("Unexpected iron duration",total))
    converted={field:before[field]*total//before["CookTotal"] for field in ("BurnTime","BurnTotal","CookTime")}
    for field,value in converted.items():
        require(s.integer(pos,field)==value,(field,"Upgrade did not preserve proportional work",before,converted))
    require(metadata("CustomName")==named and metadata("lock")==locked,"Upgrade discarded name or lock")
    s.sprint(1)
    require(s.integer(pos,"CookTime")==converted["CookTime"]+1,
            "First modded tick reset the vanilla recipe/input fingerprint")
    require(s.integer(pos,"BurnTime")==converted["BurnTime"]-1,"First modded tick changed paid fuel")
    s.sprint(total-converted["CookTime"]-1)
    s.expect_slot(pos,0,"minecraft:raw_iron",2)
    s.expect_slot(pos,1,"minecraft:coal",1)
    s.expect_slot(pos,2,"minecraft:iron_ingot",3)
    require(metadata("CustomName")==named and metadata("lock")==locked,"Continued cooking lost name or lock")
    return {"vanilla":before,"converted":converted,"inventory":"preserved",
            "first_tick":"continued partial recipe","name_and_lock":"preserved"}


def factory_lanes(s):
    pos = (4, 200, 2)
    found = {}
    for tier, ticks, inputs in [("iron",160,[9,10]), ("gold",120,list(range(8,12))), ("diamond",80,list(range(7,13)))]:
        s.machine(pos, tier, "factory")
        s.merge(pos, energy=40_000)
        for slot in inputs:
            s.replace(pos, slot, "minecraft:raw_iron")
        if len(inputs) < 6:
            s.replace(pos, 7, "minecraft:raw_iron")
        s.sprint(ticks-1)
        require(s.count(pos,"minecraft:iron_ingot") == 0, (tier, "factory completed early"))
        s.sprint(1)
        for slot in inputs:
            s.expect_slot(pos, slot, "minecraft:raw_iron", 0)
            s.expect_slot(pos, slot+6, "minecraft:iron_ingot", 1)
        require(s.integer(pos,"energy") == 40_000 - 4_000*len(inputs), (tier,"wrong exact energy debit"))
        if len(inputs) < 6:
            s.expect_slot(pos,7,"minecraft:raw_iron",1)
        found[tier] = {"lanes":len(inputs),"ticks":ticks,"energy_per_lane":4000}
    for augment,ticks,cost in [("speed",80,8000),("fuel",200,2000)]:
        s.machine(pos,"iron","factory",augment)
        s.merge(pos,energy=40_000)
        s.replace(pos,9,"minecraft:raw_iron")
        s.sprint(ticks)
        s.expect_slot(pos,15,"minecraft:iron_ingot",1)
        require(s.integer(pos,"energy") == 40_000-cost,(augment,"factory augment cost"))
        found[augment] = {"ticks":ticks,"energy":cost}
    return found


def generator_economics(s):
    pos=(4,200,2)
    found={}
    for augment,total,rate,ticks in [(None,32000,40,800),("speed",16000,80,200),("fuel",64000,30,2134)]:
        s.machine(pos,"iron","generator",augment)
        s.replace(pos,6,"minecraft:coal")
        s.sprint(1)
        require(s.integer(pos,"energy") == rate,(augment,"generation rate"))
        require(s.integer(pos,"GeneratorTotal") == total,(augment,"fuel energy budget"))
        s.sprint(ticks)
        require(s.integer(pos,"energy") == total,(augment,"finite fuel yield"))
        require(s.integer(pos,"GeneratorRemaining") == 0,(augment,"fuel budget not exhausted"))
        require("lit=false" in s.call("block",*pos)["state"],(augment,"empty generator stayed lit"))
        s.expect_slot(pos,6,"minecraft:coal",0)
        found[augment or "normal"]={"energy":total,"rate":rate}
    # The last, partially fitting production tick may not lose or create FE.
    s.machine(pos,"iron","generator")
    s.merge(pos,energy=79999)
    s.replace(pos,6,"minecraft:coal")
    s.sprint(1)
    require(s.integer(pos,"energy")==80000,"Generator overflowed capacity")
    require(s.integer(pos,"GeneratorRemaining")==31999,"Partial production lost fuel")
    s.sprint(20)
    require(s.integer(pos,"GeneratorRemaining")==31999,"Full generator kept burning")
    s.merge(pos,energy=0)
    s.sprint(800)
    require(s.integer(pos,"energy")==31999,"Resumed generator changed remaining energy")
    found["capacity_pause"]="conserved exactly"
    return found


def factory_safety(s):
    pos=(4,200,2)
    s.machine(pos,mode="factory")
    s.merge(pos,energy=8000,AutoSplit=1)
    s.replace(pos,9,"minecraft:raw_iron",64)
    s.sprint(1)
    for slot in (9,10): s.expect_slot(pos,slot,"minecraft:raw_iron",32)
    s.sprint(159)
    for slot in (9,10):
        s.expect_slot(pos,slot,"minecraft:raw_iron",31)
        s.expect_slot(pos,slot+6,"minecraft:iron_ingot",1)
    require(s.integer(pos,"energy")==0,"Auto split duplicated available energy")
    s.machine(pos,mode="factory")
    s.merge(pos,AutoSplit=1)
    for slot,count,marker in [(9,5,1),(10,3,2)]:
        s.replace(pos,slot,f'minecraft:raw_iron[minecraft:custom_data={{marker:{marker}}}]',count)
    s.sprint(1)
    for slot,count,marker in [(9,5,1),(10,3,2)]:
        s.expect_slot(pos,slot,"minecraft:raw_iron",count)
        require(s.integer(pos,f'Items[{{Slot:{slot}b}}].components."minecraft:custom_data".marker')==marker,
                "Auto split merged distinct item components")
    s.machine(pos,mode="factory")
    s.merge(pos,energy=4000)
    s.replace(pos,9,"minecraft:raw_iron")
    s.replace(pos,15,"minecraft:iron_ingot",64)
    s.sprint(160)
    require(s.integer(pos,"energy")==4000,"Blocked factory output consumed energy")
    s.expect_slot(pos,9,"minecraft:raw_iron",1)
    s.expect_slot(pos,15,"minecraft:iron_ingot",64)
    return {"auto_split":"balanced without merging components","blocked_output":"input and energy retained"}


def power_reload(s):
    generator,factory=(4,200,2),(10,200,2)
    s.machine(generator,mode="generator")
    s.machine(factory,mode="factory")
    s.replace(generator,6,"minecraft:coal")
    s.merge(factory,energy=8000)
    s.replace(factory,9,"minecraft:raw_iron",2)
    s.sprint(57)
    fields={generator:("energy","GeneratorTotal","GeneratorRemaining"),
            factory:("energy","LaneProgress2","LaneTotal2","LaneCost2","LaneReserved2")}
    def snapshot():
        return {str(pos):{key:s.integer(pos,key) for key in keys} for pos,keys in fields.items()}
    before=snapshot()
    s.call("save")
    s.stop()
    s.start()
    require(snapshot()==before,"Restart changed generator fuel or factory reservation")
    s.sprint(103)
    s.expect_slot(factory,15,"minecraft:iron_ingot",1)
    require(s.integer(factory,"energy")==4000,"Resumed factory changed operation cost")
    require(s.integer(generator,"energy")==6400,"Resumed generator changed production rate")
    return {"saved":before,"restart":"exact budgets and progress retained","continued":"factory and generator"}


def special_generator_fuels(s):
    pos=(4,200,2)
    found={}
    for mode,item,total in [("smoking","minecraft:beef",48000),("blasting","minecraft:redstone",10000)]:
        s.machine(pos,"iron","generator")
        s.interact(pos,f"ironfurnaces:augment_{mode}")
        s.sprint(1)
        s.replace(pos,6,item)
        s.sprint(1)
        require(s.integer(pos,"GeneratorTotal")==total,(mode,"generator recipe budget"))
        s.sprint(1300)
        require(s.integer(pos,"energy")==total,(mode,"generator total yield"))
        found[mode]=total
    s.machine(pos,"iron","generator")
    s.replace(pos,6,"ironfurnaces:rainbow_coal[minecraft:damage=5119]")
    s.sprint(101)
    s.expect_slot(pos,6,"ironfurnaces:rainbow_coal",0)
    require(s.integer(pos,"energy")==4000,"Last rainbow coal use did not yield4000 FE")
    found["rainbow_coal_final_use"]=4000
    return found


def generator_factory_link(s):
    factory=(4,200,2)
    generator=(5,200,2)
    s.machine(factory,"iron","factory")
    s.machine(generator,"iron","generator")
    s.merge(factory,Side5=1)
    s.merge(generator,Side4=2)
    s.replace(generator,6,"minecraft:coal")
    for slot in (9,10): s.replace(factory,slot,"minecraft:raw_iron")
    s.sprint(1000)
    for slot in (15,16): s.expect_slot(factory,slot,"minecraft:iron_ingot",1)
    require(s.integer(generator,"energy") + s.integer(factory,"energy") == 24000,
            "Generator-to-factory transfer lost or created FE")
    return {"generated":32000,"two_operations":8000,"remaining":24000}


def automation(s):
    pos,source,destination=(4,200,2),(3,200,2),(5,200,2)
    s.machine(pos)
    s.setblock(source,"minecraft:chest")
    s.setblock(destination,"minecraft:chest")
    s.call("container",*source,"minecraft:raw_iron",8)
    s.replace(pos,1,"minecraft:coal")
    s.merge(pos,Side4=0,Side5=2,AutoInput=1,AutoOutput=1)
    s.sprint(10)
    require(s.count(source,"minecraft:raw_iron")==8,"Disabled face pulled input")
    s.merge(pos,Side4=1)
    s.sprint(2000)
    require(s.count(source,"minecraft:raw_iron")==0,"Input chest was not pulled")
    require(s.count(destination,"minecraft:iron_ingot")==8,"Output chest did not receive all results")
    require(s.count(pos,"minecraft:iron_ingot")==0,"Output remained in machine")
    # Distinct component stacks cannot be merged by auto-pull.
    s.machine(pos)
    s.replace(source,0,'minecraft:raw_iron[minecraft:custom_data={marker:1}]',32)
    s.replace(source,1,'minecraft:raw_iron[minecraft:custom_data={marker:2}]',32)
    s.merge(pos,Side4=1,AutoInput=1)
    s.sprint(1)
    s.expect_slot(pos,0,"minecraft:raw_iron",32)
    require(s.integer(pos,'Items[{Slot:0b}].components."minecraft:custom_data".marker')==1,"Pulled component lost")
    s.expect_slot(source,1,"minecraft:raw_iron",32)
    s.machine(pos)
    s.merge(pos,Side4=0,Side5=4,AutoOutput=1)
    for fuel in ("minecraft:coal","ironfurnaces:rainbow_coal","ironfurnaces:item_heater"):
        s.replace(pos,1,fuel)
        s.sprint(3)
        s.expect_slot(pos,1,fuel,1)
        require(s.count(destination,fuel)==0,(fuel,"Auto-output ejected valid fuel"))
    s.replace(pos,1,"minecraft:water_bucket")
    s.sprint(1)
    require(s.count(destination,"minecraft:water_bucket")==1,"Auto-output retained spent water bucket")
    return {"disabled_face":"blocked","auto_pull_push":8,"component_separation":"preserved",
            "fuel_auto_output":"retains fuel and receiver, extracts spent bucket"}


def hopper_routes(s):
    pos,hopper,chest=(10,202,5),(10,201,5),(10,200,5)
    s.setblock(chest,"minecraft:chest")
    s.setblock(hopper,"minecraft:hopper[facing=down]")
    for mode,slot in [(None,1),("generator",6)]:
        s.machine(pos,mode=mode)
        s.merge(pos,Side0=4,RedstoneMode=1)
        s.replace(pos,slot,"minecraft:coal")
        s.sprint(30)
        s.expect_slot(pos,slot,"minecraft:coal",1)
        require(s.count(hopper,"minecraft:coal")+s.count(chest,"minecraft:coal")==0,
                (mode,"Hopper drained unburned fuel"))
        s.replace(pos,slot,"minecraft:bucket")
        before=s.count(chest,"minecraft:bucket")
        s.sprint(30)
        s.expect_slot(pos,slot,"minecraft:bucket",0)
        require(s.count(chest,"minecraft:bucket")==before+1,(mode,"Hopper failed to extract empty bucket"))
    s.machine(pos,mode="factory")
    s.merge(pos,Side0=2)
    s.replace(pos,13,"minecraft:diamond")
    s.replace(pos,15,"minecraft:iron_ingot")
    s.sprint(30)
    s.expect_slot(pos,13,"minecraft:diamond",1)
    s.expect_slot(pos,15,"minecraft:iron_ingot",0)
    require(s.count(chest,"minecraft:iron_ingot")==1,"Factory active output was not exposed to hopper")
    return {"fuel_faces":"bucket-only extraction","factory_outputs":"active lanes only"}


def redstone(s):
    pos=(4,200,2)
    powered=(4,200,3)
    s.setblock(powered,"minecraft:air")
    s.machine(pos)
    s.replace(pos,0,"minecraft:raw_iron",2)
    s.replace(pos,1,"minecraft:coal",2)
    s.merge(pos,RedstoneMode=1)
    s.sprint(200)
    s.expect_slot(pos,1,"minecraft:coal",2)
    s.expect_slot(pos,2,"minecraft:iron_ingot",0)
    s.setblock(powered,"minecraft:redstone_block")
    s.sprint(160)
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    s.merge(pos,RedstoneMode=2)
    s.sprint(200)
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    s.setblock(powered,"minecraft:air")
    s.sprint(160)
    s.expect_slot(pos,2,"minecraft:iron_ingot",2)
    # A single full generator fuel slot is comparator15; subtraction is12.
    s.machine(pos,"iron","generator")
    s.replace(pos,6,"minecraft:stone",64)
    s.merge(pos,RedstoneMode=3)
    s.sprint(1)
    require(s.integer(pos,"LastComparator")==15,"Comparator counted inactive slots")
    require(s.integer(pos,"LastSignal")==15,"Comparator-output mode emitted wrong strength")
    s.merge(pos,RedstoneMode=4,RedstoneValue=3)
    s.sprint(1)
    require(s.integer(pos,"LastSignal")==12,"Comparator subtraction failed")
    return {"requires_power":"passed","requires_no_power":"passed","comparator":15,"subtracted":12}


def copy_and_reload(s):
    source,destination=(4,200,2),(6,200,2)
    s.machine(source)
    s.machine(destination)
    s.setblock(source,"ironfurnaces:iron_furnace[facing=west]")
    settings={"Side0":2,"Side1":1,"Side2":3,"Side3":0,"Side4":4,"Side5":1,
              "AutoInput":1,"AutoOutput":1,"RedstoneMode":4,"RedstoneValue":7,"AugmentTab":1,"AutoSplit":1}
    s.merge(source,**settings)
    facing=re.search(r"facing=(north|south|east|west)",s.call("block",*source)["state"]).group(1)
    require(facing=="west","Copy facing fixture did not rotate")
    s.interact(source,"ironfurnaces:item_copy")
    inventory=s.call("inventory")
    require("ironfurnaces:settings" in inventory["hotbar"][0].get("components",""),"Copy tool has no saved settings")
    require(inventory["counts"].get("ironfurnaces:item_copy")==1,"Copying settings duplicated the tool")
    s.use_held(destination,sneak=True)
    for key,value in settings.items(): require(s.integer(destination,key)==value,(key,"copy/apply mismatch"))
    require(f"facing={facing}" in s.call("block",*destination)["state"],"Copied facing not applied")
    before={key:s.integer(destination,key) for key in settings}
    s.call("save")
    s.stop()
    s.start()
    after={key:s.integer(destination,key) for key in settings}
    require(after==before,("Settings changed after restart",before,after))
    return {"copied":before,"facing":facing,"restart":"preserved"}


def wireless(s):
    heater,furnace=(8,200,2),(10,200,2)
    s.setblock(heater,"minecraft:air")
    s.setblock(heater,"ironfurnaces:heater")
    s.sprint(1)
    require(len(s.slots(heater))==1,"Heater inventory did not initialize")
    s.merge(heater,energy=2000)
    s.replace(heater,0,"ironfurnaces:item_heater")
    s.sprint(1)
    for field,value in zip(("HeaterPosX","HeaterPosY","HeaterPosZ"),heater):
        require(s.integer(heater,f'Items[{{Slot:0b}}].components."minecraft:custom_data".{field}')==value,"Receiver did not bind heater position")
    s.machine(furnace)
    s.command("item replace block 10 200 2 container.1 from block 8 200 2 container.0")
    s.replace(heater,0,"minecraft:air")
    s.replace(furnace,0,"minecraft:raw_iron")
    s.sprint(160)
    s.expect_slot(furnace,2,"minecraft:iron_ingot",1)
    s.expect_slot(furnace,1,"ironfurnaces:item_heater",1)
    require(s.integer(heater,"energy")==0,"Wireless operation did not debit exactly2000 FE")
    return {"binding":"heater coordinates","cost":2000,"receiver":"retained"}


def wireless_unloaded(s):
    heater,furnace=(1024,200,2),(10,200,2)
    s.machine(furnace)
    s.call("tp",1024.5,200,.5)
    s.wait_loaded(heater)
    s.setblock(heater,"minecraft:air")
    s.setblock(heater,"ironfurnaces:heater")
    s.sprint(1)
    require(s.call("block",*heater)["block"]=="ironfurnaces:heater","Remote heater placement failed")
    require(len(s.slots(heater))==1,"Remote heater inventory did not initialize")
    s.merge(heater,energy=2000)
    s.replace(heater,0,"ironfurnaces:item_heater")
    s.sprint(1)
    require(s.integer(heater,'Items[{Slot:0b}].components."minecraft:custom_data".HeaterPosX')==1024,
            "Remote receiver did not bind")
    s.command("item replace entity @s hotbar.0 from block 1024 200 2 container.0")
    s.replace(heater,0,"minecraft:air")
    s.call("tp",10.5,200,.5)
    s.wait_loaded(furnace)
    s.command("item replace block 10 200 2 container.1 from entity @s hotbar.0")
    s.expect_slot(furnace,1,"ironfurnaces:item_heater",1)
    s.command("clear @s")
    s.call("save")
    s.stop()
    s.start()
    s.sprint(1)
    require(not s.call("block",*heater)["loaded"],"Remote wireless fixture did not start unloaded")
    s.replace(furnace,0,"minecraft:raw_iron")
    for _ in range(60):
        s.sprint(20)
        if s.count(furnace,"minecraft:iron_ingot"):
            break
    s.expect_slot(furnace,2,"minecraft:iron_ingot",1)
    s.expect_slot(furnace,1,"ironfurnaces:item_heater",1)
    # Revisit the source even if its temporary loading ticket was released.
    s.call("tp",1024.5,200,.5)
    s.wait_loaded(heater)
    require(s.integer(heater,"energy")==0,"Remote wireless did not debit exactly2000 FE")
    s.call("tp",.5,200,.5)
    s.wait_loaded(furnace)
    return {"source_distance":1014,"initial_chunk":"unloaded","cost":2000,"result":"one smelt"}


def rainbow_network(s):
    owner=s.owner()
    for index,tier in enumerate(("copper","iron","silver","gold","diamond","emerald","crystal","obsidian","netherite")):
        pos=(index,200,10)
        s.machine(pos,tier,"generator")
        s.merge(pos,Owner=owner)
        s.replace(pos,6,"minecraft:coal",64)
    rainbow,heater=(12,200,10),(13,200,10)
    s.machine(rainbow,"million","generator")
    s.merge(rainbow,Owner=owner,Side5=2)
    s.setblock(heater,"minecraft:air")
    s.setblock(heater,"ironfurnaces:heater")
    s.sprint(10)
    require(s.integer(rainbow,"RainbowGen")==1,"Complete owned network did not activate rainbow output")
    energy=s.integer(heater,"energy")
    require(energy>=50000,("Rainbow generator did not emit bonus energy",energy))
    s.merge((0,200,10),RedstoneMode=1)
    s.sprint(3)
    require(s.integer(rainbow,"RainbowGen")==0,"Missing running tier did not disable rainbow output")
    stopped=s.integer(heater,"energy")
    s.sprint(3)
    require(s.integer(heater,"energy")==stopped,"Inactive rainbow network kept producing energy")
    return {"nine_required_tiers":"activated","bonus_energy":energy,"stopped_tier":"disabled"}


def modes_and_skins(s):
    pos=(4,200,2)
    s.machine(pos)
    s.interact(pos,"ironfurnaces:augment_smoking")
    s.sprint(1)
    require("type=1" in s.call("block",*pos)["state"],"Smoking augment did not select smoker model")
    s.interact(pos,"ironfurnaces:augment_factory")
    s.sprint(1)
    require("type=1" in s.call("block",*pos)["state"],"Factory mode replaced recipe-mode model")
    for item,skin in [("item_spooky",1),("item_xmas",2)]:
        s.interact(pos,f"ironfurnaces:{item}")
        s.sprint(1)
        state=s.call("block",*pos)["state"]
        require(f"jovial={skin}" in state and "type=1" in state,(item,"seasonal smoker state",state))
    s.interact(pos,None,sneak=True)
    require("jovial=0" in s.call("block",*pos)["state"],"Skin reset did not restore tier artwork")
    s.expect_slot(pos,3,"ironfurnaces:augment_smoking",1)
    s.expect_slot(pos,5,"ironfurnaces:augment_factory",1)
    return {"recipe_model":"smoking","factory_mode":"preserves recipe model","seasonal_skins":"applied and reset"}


CASES={"migration":migration,"vanilla_upgrade":vanilla_upgrade,
       "factory_lanes":factory_lanes,"factory_safety":factory_safety,
       "generator_economics":generator_economics,"power_reload":power_reload,
       "special_fuels":special_generator_fuels,"energy_link":generator_factory_link,
       "automation":automation,"hopper_routes":hopper_routes,"redstone":redstone,"wireless":wireless,
       "wireless_unloaded":wireless_unloaded,
       "rainbow_network":rainbow_network,"modes_skins":modes_and_skins,"copy_reload":copy_and_reload}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ("game","ctl","resources","run-dir"):
        parser.add_argument(f"--{name}",type=Path,required=True)
    parser.add_argument("--content",type=Path,default=ROOT/".build/migration-v3/mod")
    parser.add_argument("--windowed",action="store_true")
    parser.add_argument("--offscreen",action="store_true")
    parser.add_argument("--keep-going",action="store_true")
    parser.add_argument("--only",default=",".join(CASES))
    args=parser.parse_args()
    for key in ("game","ctl","resources","content"): setattr(args,key,getattr(args,key).resolve(strict=True))
    if args.run_dir.exists(): parser.error("--run-dir must be fresh")
    if args.run_dir.resolve().is_relative_to(args.resources): parser.error("run directory must be outside resources")
    if args.offscreen and not args.windowed: parser.error("--offscreen requires --windowed")
    selected=args.only.split(",")
    if not all(name in CASES for name in selected): parser.error("unknown --only case")
    session=PowerSession(args)
    try:
        session.start()
        for name in selected:
            session.call("close")
            session.command("tick freeze")
            session.call("fill",-4,200,-4,23,204,15,"minecraft:air")
            try:
                session.run_case(name,CASES[name])
            except Exception:
                if not args.keep_going: raise
    finally:
        try: session.stop()
        finally: session.write()
    failed=sum(value["status"]=="failed" for value in session.results.values())
    print(f"Passed {len(session.results)-failed}; failed {failed}: {session.run/'results.json'}",flush=True)
    if failed: raise SystemExit(1)


if __name__ == "__main__": main()
