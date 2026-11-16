"""Real integration checks in a fresh disposable Horizon world.

Default checks are headless. Use --windowed --only menu to exercise actual
menu clicks: Horizon's click-slot needs a renderer. No existing saves, launcher
settings or prepared resources are changed. Headless checks do not verify art.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
TIERS = [("copper",180,1),("iron",160,1),("silver",140,1),("gold",120,1),
         ("diamond",80,1),("emerald",40,1),("crystal",40,1),("obsidian",20,1),
         ("netherite",5,1),("million",20,64),("allthemodium",5,16),
         ("vibranium",3,32),("unobtainium",1,64)]
WORLD = "IronFurnacesPortSmoke"


def require(condition, detail):
    if not condition:
        raise AssertionError(detail)


def numeric_reply(command, reply):
    # Match the actual 26.3 en_us messages, not any final integer: an error
    # can contain a coordinate or NBT slot index that is not a query result.
    # data.get without a scale uses commands.data.block.query even when a
    # path selects one integer. Integer NBT suffixes are optional.
    if command=="time query gametime":
        pattern=r"The game time is (-?\d+) tick\(s\)\s*$"
    elif command.startswith("data get block "):
        pattern=r"has the following block data:\s*(-?\d+)(?:[bBsSlL])?\s*$"
    else:
        raise ValueError(f"Unsupported numeric query: {command}")
    for line in reversed(reply.get("chat", [])):
        value=re.search(pattern,line)
        if value:
            return int(value.group(1))
    raise AssertionError(("No numeric command result",command,reply))


class Session:
    def __init__(self, args):
        self.args = args
        self.run = args.run_dir.resolve()
        self.run.mkdir(parents=True, exist_ok=False)
        self.game_dir = self.run / "game"
        self.game_dir.mkdir()
        self.endpoint = self.run / "control.json"
        self.env = dict(os.environ)
        for name in ("HORIZON_SESSION_FILE", "HORIZON_CONTROL"):
            self.env.pop(name, None)
        for name in ("HORIZON_HOME", "HORIZON_LAUNCHER_HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR", "HORIZON_LAUNCHER_TRASH", "HORIZON_LAUNCHER_CLIPBOARD", "HORIZON_LAUNCHER_DESKTOP"):
            path = self.run / name.lower()
            path.mkdir()
            self.env[name] = str(path)
        self.env.update(HORIZON_LAUNCHER_NO_SELF_UPDATE="1", HORIZON_NO_AUDIO="1",
                        HORIZON_NO_DISCORD="1", HORIZON_STILL="1", HORIZON_CONTROL=str(self.endpoint))
        self.options = dict(cwd=self.game_dir, env=self.env)
        if os.name == "nt":
            self.options["creationflags"] = subprocess.CREATE_NO_WINDOW
        self.process = None
        self.generation = 0
        self.evidence = []
        self.results = {}

    def call(self, *words, timeout=20):
        result = subprocess.run([str(self.args.ctl), "--socket", str(self.endpoint),
                                 "--timeout", str(timeout), *map(str, words)],
                                capture_output=True, text=True, encoding="utf-8",
                                errors="replace", timeout=timeout+3, **self.options)
        if result.returncode:
            self.evidence.append({"command":words,"exit_code":result.returncode,
                                  "stdout":result.stdout,"stderr":result.stderr})
            raise RuntimeError(f"Control {words!r} failed: {result.stderr} {result.stdout}")
        value = json.loads(result.stdout)
        self.evidence.append({"command": words, "reply": value})
        if isinstance(value, dict) and value.get("error"):
            raise RuntimeError((words, value))
        return value

    def command(self, text):
        return self.call("command", text)

    def numeric(self, text):
        return numeric_reply(text,self.command(text))

    def integer(self, pos, field):
        return self.numeric(f"data get block {' '.join(map(str,pos))} {field}")

    def sprint(self, ticks):
        before = self.numeric("time query gametime")
        self.command(f"tick sprint {ticks}")
        deadline = time.monotonic()+30
        while time.monotonic() < deadline:
            now = self.numeric("time query gametime")
            if now >= before+ticks:
                require(now == before+ticks, ("World did not remain frozen", before,ticks,now))
                return
            time.sleep(.05)
        raise TimeoutError(f"World did not advance {ticks} ticks")

    def setblock(self, pos, block):
        reply=self.call("setblock", *pos, block)
        if block.startswith("ironfurnaces:") and "_furnace" in block:
            # A newly created ModBlockEntity has an empty items Vec. Horizon
            # sizes it to ModContainer.slots on its first tick or menu open;
            # the container debug command cannot insert until then. Warm an
            # EMPTY furnace before supplying any input/fuel, so the measured
            # recipe ticks still start with the first call after seeding.
            if not self.slots(pos):
                self.sprint(1)
            require(len(self.slots(pos)) in (3,19),("Furnace container did not initialize",pos,block))
        return reply

    def slots(self, pos):
        return self.call("container", *pos)["slots"]

    def slot(self, pos, index):
        return self.slots(pos)[index]

    def seed(self, pos, tier="iron", count=9, fuel="minecraft:coal"):
        self.setblock(pos, "minecraft:air")
        self.setblock(pos, f"ironfurnaces:{tier}_furnace")
        self.call("container", *pos, "minecraft:raw_iron", count)
        self.call("container", *pos, fuel, 1)

    def expect_slot(self, pos, index, item, count):
        actual = self.slot(pos,index)
        expected = None if count == 0 else {"item":item,"count":count}
        require(actual == expected, (pos,index,expected,actual))

    def held(self, item):
        self.command("clear @s")
        if item:
            self.command(f"item replace entity @s hotbar.0 with {item} 1")
        self.call("select", 1)

    def interact(self, pos, item=None, sneak=False, close=True):
        self.call("close")
        self.call("tp", pos[0]+.5, pos[1], pos[2]-2)
        self.held(item)
        words = ["interact", *pos, "north"]
        if sneak:
            # The game's keyboard-latch repair clears the raw Shift held by
            # interact --sneak. A held controller stick button is a supported
            # input source and survives that repair. Release and untoggle it
            # even if the interaction/assertion fails.
            self.call("pad","rs","down")
            try:
                self.call("wait",5)
                state=self.call("state")
                require(state["player"]["sneaking"],("Controller did not engage sneak",state))
                reply=self.call(*words)
            finally:
                try:
                    self.call("pad","rs","up")
                    if close:
                        self.call("close")
                    self.call("wait",3)
                    if self.call("state")["player"]["sneaking"]:
                        self.call("pad","rs","tap")
                        self.call("wait",3)
                finally:
                    self.call("pad","release")
                require(not self.call("state")["player"]["sneaking"],"Sneak cleanup left the player crouching")
        else:
            reply = self.call(*words)
        if close:
            self.call("close")
        return reply

    def start(self):
        self.generation += 1
        if self.endpoint.exists():
            self.endpoint.unlink()
        args = [str(self.args.game), "--resources", str(self.args.resources),
                "--game-dir", str(self.game_dir), "--control-socket", str(self.endpoint),
                "--dev-mod", str(self.args.content), "--world", WORLD, "--seed", "5",
                "--creative", "--name", "FurnaceTest", "--no-pause",
                "--render-distance", "2", "--timeout-seconds", "1200"]
        if not self.args.windowed:
            args.append("--headless")
        elif self.args.offscreen:
            args.append("--offscreen")
        with (self.run / f"client-{self.generation}.log").open("w",encoding="utf-8") as log:
            self.process = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, **self.options)
        deadline = time.monotonic()+180
        while not self.endpoint.is_file():
            if self.process.poll() is not None:
                raise RuntimeError(f"Game exited at startup: {self.process.returncode}")
            if time.monotonic() >= deadline:
                raise TimeoutError("Game did not create control endpoint")
            time.sleep(.1)
        self.call("wait-ready", 60, timeout=65)
        state = self.call("state")
        require(state.get("world") == WORLD, state)
        self.command("tick freeze")
        self.command("gamerule spawn_mobs false")
        self.command("gamerule random_tick_speed 0")
        self.command("gamemode creative")
        self.call("tp", .5,200,.5)
        self.call("wait-ready", 60, timeout=65)
        self.call("fill", -5,199,-5, 24,199,16, "minecraft:bedrock")
        self.call("tp", .5,200,.5)

    def stop(self):
        if self.process is None or self.process.poll() is not None:
            return
        try:
            self.call("quit")
            self.process.wait(timeout=30)
        finally:
            if self.process.poll() is None:
                self.process.terminate()
                self.process.wait(timeout=5)
        require(self.process.returncode == 0, ("Game exit",self.process.returncode))

    def run_case(self, name, function):
        print(f"Running {name}", flush=True)
        try:
            detail = function(self)
            self.results[name] = {"status":"passed", "detail":detail}
        except Exception as error:
            self.results[name] = {"status":"failed", "error":str(error)}
            raise
        finally:
            self.write()

    def write(self):
        (self.run / "results.json").write_text(json.dumps(self.results,indent=2)+"\n",encoding="utf-8")
        (self.run / "evidence.json").write_text(json.dumps(self.evidence,indent=2)+"\n",encoding="utf-8")


def timings(s):
    found = {}
    pos = (4,200,2)
    for tier,ticks,batch in TIERS:
        s.seed(pos,tier,64)
        if ticks>1:
            s.sprint(ticks-1)
            s.expect_slot(pos,2,"minecraft:iron_ingot",0)
        s.sprint(1)
        s.expect_slot(pos,2,"minecraft:iron_ingot",batch)
        s.expect_slot(pos,0,"minecraft:raw_iron",64-batch)
        found[tier] = {"ticks":ticks,"first_batch":batch}
    return found


def economics(s):
    positions = []
    for index,(tier,ticks,batch) in enumerate(TIERS):
        if batch == 1:
            pos = (index,200,5)
            s.seed(pos,tier,9)
            positions.append(pos)
    s.sprint(2000)
    for pos in positions:
        s.expect_slot(pos,2,"minecraft:iron_ingot",8)
        s.expect_slot(pos,0,"minecraft:raw_iron",1)
        s.expect_slot(pos,1,"minecraft:coal",0)
        require(s.integer(pos,"BurnTime") == 0, pos)
    return {"ordinary_tiers":len(positions),"outputs_per_coal":8}


def augments(s):
    pos=(4,200,2)
    result={}
    for augment, expected in (("speed",4),("fuel",16)):
        s.seed(pos,"iron",32)
        s.interact(pos,f"ironfurnaces:augment_{augment}")
        s.sprint(4000)
        s.expect_slot(pos,2,"minecraft:iron_ingot",expected)
        result[augment]=expected
    for augment,material,result_item,recipe_name in (
        ("smoking","minecraft:beef","minecraft:cooked_beef","cooked_beef_from_smoking"),
        ("blasting","minecraft:raw_iron","minecraft:iron_ingot","iron_ingot_from_blasting_raw_iron"),
    ):
        recipe=json.loads((s.args.resources / "data/minecraft/recipe" / f"{recipe_name}.json").read_text(encoding="utf-8"))
        require(recipe["type"]==f"minecraft:{augment}",(recipe_name,recipe))
        ticks=max(1,160*recipe["cookingtime"]//200)
        s.setblock(pos,"minecraft:air")
        s.setblock(pos,"ironfurnaces:iron_furnace")
        s.interact(pos,f"ironfurnaces:augment_{augment}")
        if len(s.slots(pos)) == 19:
            s.expect_slot(pos,3,f"ironfurnaces:augment_{augment}",1)
        else:
            mode=s.command("data get block 4 200 2 Mode")
            require(any(f'"ironfurnaces:augment_{augment}"' in line for line in mode.get("chat",[])),
                    ("Recipe mode augment was not installed",augment,mode))
        s.call("container",*pos,material,1)
        s.call("container",*pos,"minecraft:coal",1)
        if ticks>1:
            s.sprint(ticks-1)
        s.expect_slot(pos,2,result_item,0)
        s.sprint(1)
        s.expect_slot(pos,2,result_item,1)
        result[augment]={"recipe_ticks":recipe["cookingtime"],"furnace_ticks":ticks,"mode_installed":True}
    return result


def upgrades(s):
    pos=(4,200,2)
    s.seed(pos,"iron",9)
    s.sprint(5)
    before=s.slots(pos)
    burn=s.integer(pos,"BurnTime")
    burn_total=s.integer(pos,"BurnTotal")
    progress=s.integer(pos,"CookTime")
    cook_total=s.integer(pos,"CookTotal")
    s.interact(pos,"ironfurnaces:upgrade_gold")
    require(s.call("block",*pos)["block"]=="ironfurnaces:gold_furnace","upgrade did not change block")
    # The new API reloads sparse saved inventories; trailing empty slots are
    # expanded on the first tick/menu use. Compare contents before that tick.
    after=s.slots(pos)
    require(after+[None]*(len(before)-len(after))==before,"upgrade lost inventory")
    after_burn=s.integer(pos,"BurnTime")
    after_burn_total=s.integer(pos,"BurnTotal")
    after_progress=s.integer(pos,"CookTime")
    after_cook_total=s.integer(pos,"CookTotal")
    require(abs(after_burn/after_burn_total-burn/burn_total)<=1/after_burn_total,"upgrade lost fuel fraction")
    require(abs(after_progress/after_cook_total-progress/cook_total)<=1/after_cook_total,"upgrade lost cooking fraction")
    s.sprint(1)
    require(s.slots(pos)==before,"upgrade did not restore the full container")
    require(s.integer(pos,"CookTime")==after_progress+1,"upgrade reset the partial recipe")
    s.interact(pos,"ironfurnaces:upgrade_copper")
    require(s.call("block",*pos)["block"]=="ironfurnaces:gold_furnace","invalid upgrade changed block")
    s.setblock(pos,"minecraft:furnace")
    s.interact(pos,"ironfurnaces:upgrade_iron")
    require(s.call("block",*pos)["block"]=="ironfurnaces:iron_furnace","vanilla upgrade failed")
    # After its last input completes the furnace is empty but still burning.
    # A tier change must preserve paid operations, not retain slower-tier ticks.
    s.seed(pos,"iron",1)
    s.sprint(161)
    s.expect_slot(pos,0,"minecraft:raw_iron",0)
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    empty_burn=s.integer(pos,"BurnTime")
    empty_total=s.integer(pos,"BurnTotal")
    require(empty_burn>0,"Empty-upgrade fixture did not retain paid fuel")
    s.interact(pos,"ironfurnaces:upgrade_gold")
    require(s.call("block",*pos)["block"]=="ironfurnaces:gold_furnace","empty upgrade did not change block")
    require(s.integer(pos,"BurnTime")==empty_burn*120//160,"Empty upgrade did not rescale remaining paid fuel")
    require(s.integer(pos,"BurnTotal")==empty_total*120//160,"Empty upgrade did not rescale total paid fuel")
    s.expect_slot(pos,0,"minecraft:raw_iron",0)
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    return {"modded":"iron->gold", "vanilla":"furnace->iron", "invalid":"rejected", "inventory_and_work":"preserved",
            "empty_burning_upgrade":{"before":empty_burn,"after":s.integer(pos,"BurnTime")}}


def menu(s):
    if not s.args.windowed:
        raise RuntimeError("Use --windowed --only menu; Horizon click-slot requires a renderer")
    pos=(4,200,2)
    s.setblock(pos,"minecraft:air")
    s.setblock(pos,"ironfurnaces:iron_furnace")
    s.interact(pos,close=False)
    s.call("screenshot",str(s.run/"furnace-menu.png"))
    s.command("item replace entity @s hotbar.0 with minecraft:dirt 1")
    s.call("wait",4)
    s.call("click-slot",0)
    s.call("wait",4)
    carried=s.call("menu").get("carried")
    require(carried is not None,("No carried item after selecting dirt",carried))
    s.call("click-slot",302)
    s.call("wait",4)
    s.expect_slot(pos,2,"minecraft:dirt",0)
    require(s.call("menu").get("carried")==carried,"result slot accepted an inserted item")
    s.call("click-slot",301)
    s.call("wait",4)
    s.expect_slot(pos,1,"minecraft:dirt",0)
    require(s.call("menu").get("carried")==carried,"fuel slot accepted dirt")
    s.call("click-slot",0)
    s.call("wait",4)
    s.call("close")
    s.call("screenshot",str(s.run/"furnace-world.png"))

    # The same component-bearing item in two distinct player stacks must
    # honor its own maximum stack size when shift-clicked into the input.
    s.setblock(pos,"minecraft:air")
    s.setblock(pos,"ironfurnaces:iron_furnace")
    s.interact(pos,close=False)
    for slot in (0,1):
        s.command(f"item replace entity @s hotbar.{slot} with minecraft:raw_iron[minecraft:max_stack_size=1] 1")
    s.call("wait",4)
    inventory=s.call("inventory")
    for slot in (0,1):
        stack=inventory["hotbar"][slot]
        require(stack.get("item")=="minecraft:raw_iron" and stack.get("count")==1 and
                "max_stack_size" in stack.get("components",""),("Stack-limit fixture invalid",stack))
    s.call("click-slot",0,"shift")
    s.call("wait",4)
    s.expect_slot(pos,0,"minecraft:raw_iron",1)
    s.call("click-slot",1,"shift")
    s.call("wait",4)
    s.expect_slot(pos,0,"minecraft:raw_iron",1)
    remaining=s.call("inventory")
    require(remaining["counts"].get("minecraft:raw_iron",0)==1,("Second limited stack did not remain in player inventory",remaining))
    require(remaining["hotbar"][1].get("item")=="minecraft:raw_iron" and
            remaining["hotbar"][1].get("count")==1,("Second shift-click moved a full-slot item",remaining["hotbar"][1]))
    s.call("close")

    # Smelt real input, remove it using the real result-slot shift-click, and
    # advance the world to release and collect its banked recipe experience.
    s.seed(pos,"iron",8)
    s.sprint(1280)
    s.expect_slot(pos,2,"minecraft:iron_ingot",8)
    s.command("gamemode survival")
    s.interact(pos,close=False)
    before_xp=s.call("state")["player"]["xp"]["total"]
    s.call("click-slot",302,"shift")
    s.call("wait",4)
    s.expect_slot(pos,2,"minecraft:iron_ingot",0)
    extracted=s.call("inventory")
    require(extracted["counts"].get("minecraft:iron_ingot",0)==8,("Result shift-click did not transfer eight ingots",extracted))
    s.call("close")
    s.sprint(200)
    s.call("wait",4)
    after_xp=s.call("state")["player"]["xp"]["total"]
    require(after_xp>before_xp,("Extracted output did not award recipe XP",before_xp,after_xp))
    s.command("gamemode creative")
    return {"result_slot_insertion":"rejected","invalid_fuel":"rejected",
            "component_max_stack_size":{"input":1,"player":1},"shift_extraction":8,
            "recipe_xp":{"before":before_xp,"after":after_xp}}


def hoppers(s):
    s.setblock((12,200,10),"minecraft:chest")
    s.setblock((12,201,10),"minecraft:hopper[facing=down]")
    s.setblock((12,202,10),"ironfurnaces:copper_furnace")
    if len(s.slots((12,202,10))) == 19:
        # Set the upstream placement defaults explicitly so this regression
        # fixture also works with worlds that saved different side settings.
        s.command("data merge block 12 202 10 {Side0:2,Side1:1,Side2:4,Side3:4,Side4:4,Side5:4}")
    s.setblock((12,203,10),"minecraft:hopper[facing=down]")
    s.setblock((11,202,10),"minecraft:hopper[facing=east]")
    s.call("container",12,203,10,"minecraft:raw_iron",8)
    s.call("container",11,202,10,"minecraft:coal",1)
    s.sprint(2000)
    slots=s.slots((12,200,10))
    require(sum(slot["count"] for slot in slots if slot and slot["item"]=="minecraft:iron_ingot")==8,slots)
    require(all(not slot or slot["item"]=="minecraft:iron_ingot" for slot in slots),slots)
    return {"top_input":"passed","side_fuel":"passed","bottom_output":8}


def persistence(s):
    pos=(4,200,2)
    s.seed(pos,"iron",9)
    s.interact(pos,"ironfurnaces:augment_fuel")
    s.sprint(20)
    slot_count=len(s.slots(pos))
    def snapshot():
        slots=s.slots(pos)
        require(len(slots)<=slot_count,("Saved furnace has too many slots",slots))
        # Saved inventories omit trailing empty slots. Normalize only this
        # comparison; setblock still needs slots() to expose fresh empty Vecs.
        return {"slots":slots+[None]*(slot_count-len(slots)),"burn":s.integer(pos,"BurnTime"),"progress":s.integer(pos,"CookTime")}
    before=snapshot()
    s.call("save")
    s.stop()
    s.start()
    after=snapshot()
    require(before==after,("Reload changed saved furnace",before,after))
    s.sprint(180)
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    return {"saved":before,"reloaded":after,"continued_cooking":"passed"}


def blocked(s):
    """Blocked outputs must not spend input or ignite a new fuel item."""
    pos=(4,200,2)
    result={}
    for label,item,count in (("full","minecraft:iron_ingot",64),
                             ("different_item","minecraft:diamond",1)):
        s.seed(pos,"iron",9)
        s.call("container",*pos,item,count)
        before=s.slots(pos)
        s.sprint(400)
        require(s.slots(pos)==before,(label,"blocked furnace changed inventory",before,s.slots(pos)))
        require(s.integer(pos,"BurnTime")==0,(label,"blocked furnace ignited fuel"))
        require(s.integer(pos,"CookTime")==0,(label,"blocked furnace advanced cooking"))
        result[label]="input, fuel and output preserved"
    # Same item with a different component must not merge with recipe output.
    s.seed(pos,"iron",9)
    s.command("item replace block 4 200 2 container.2 with minecraft:iron_ingot[minecraft:custom_data={ironfurnaces_test:1}] 1")
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    require(s.integer(pos,'Items[{Slot:2b}].components."minecraft:custom_data".ironfurnaces_test')==1,
            "Could not seed output with a distinguishing component")
    before=s.slots(pos)
    s.sprint(400)
    require(s.slots(pos)==before,"Furnace merged incompatible item components")
    require(s.integer(pos,"BurnTime")==0,"Component mismatch consumed fuel")
    result["different_components"]="rejected"
    return result


def batch_capacity(s):
    pos=(4,200,2)
    s.seed(pos,"million",64)
    s.call("container",*pos,"minecraft:iron_ingot",61)
    s.sprint(20)
    s.expect_slot(pos,2,"minecraft:iron_ingot",64)
    s.expect_slot(pos,0,"minecraft:raw_iron",61)
    s.sprint(40)
    s.expect_slot(pos,2,"minecraft:iron_ingot",64)
    s.expect_slot(pos,0,"minecraft:raw_iron",61)
    return {"available_output_spaces":3,"consumed_inputs":3,"remaining_inputs":61,"blocked_next_batch":"preserved"}


def remainders(s):
    pos=(4,200,2)
    s.seed(pos,"iron",1,"minecraft:lava_bucket")
    s.sprint(1)
    s.expect_slot(pos,1,"minecraft:bucket",1)
    s.sprint(159)
    s.expect_slot(pos,2,"minecraft:iron_ingot",1)
    s.expect_slot(pos,1,"minecraft:bucket",1)
    s.setblock(pos,"minecraft:air")
    s.setblock(pos,"ironfurnaces:iron_furnace")
    s.call("container",*pos,"minecraft:wet_sponge",1)
    s.call("container",*pos,"minecraft:lava_bucket",1)
    s.sprint(1)
    s.expect_slot(pos,1,"minecraft:bucket",1)
    s.sprint(159)
    s.expect_slot(pos,0,"minecraft:wet_sponge",0)
    s.expect_slot(pos,2,"minecraft:sponge",1)
    s.expect_slot(pos,1,"minecraft:water_bucket",1)
    return {"lava_bucket":"empty bucket retained", "wet_sponge":"sponge plus water bucket"}


def augment_swap(s):
    """Do not turn paid slow fuel into additional fast smelting operations."""
    pos=(4,200,2)
    result={}
    for label,start,end,remove in (
        ("efficiency_to_speed","fuel","speed",False),
        ("mode_change",None,"blasting",False),
        ("remove_augment","fuel",None,True),
    ):
        s.seed(pos,"iron",32)
        if start:
            s.interact(pos,f"ironfurnaces:augment_{start}")
        s.sprint(20)
        before=s.integer(pos,"BurnTime")
        require(before>0,(label,"fixture did not ignite"))
        s.expect_slot(pos,1,"minecraft:coal",0)
        if remove and len(s.slots(pos))==19:
            # In the full Java interface augments are removed through their
            # menu slots. Sneak+empty hand resets the seasonal skin instead.
            s.interact(pos,"ironfurnaces:item_spooky")
            s.interact(pos,None,sneak=True)
            require(s.integer(pos,"Skin")==0,"Empty-hand sneak did not clear seasonal skin")
            s.expect_slot(pos,4,"ironfurnaces:augment_fuel",1)
            require(s.integer(pos,"BurnTime")==before,"Skin reset changed paid fuel")
            result["skin_reset"]={"augment":"retained","paid_fuel":"preserved"}
            continue
        s.interact(pos,f"ironfurnaces:augment_{end}" if end else None,sneak=remove)
        if len(s.slots(pos))==19:
            s.sprint(1)
        require(s.integer(pos,"BurnTime")==0,(label,"old paid fuel survived augment change"))
        require(s.integer(pos,"CookTime")==0,(label,"old cooking progress survived augment change"))
        s.sprint(1000)
        s.expect_slot(pos,0,"minecraft:raw_iron",32)
        s.expect_slot(pos,1,"minecraft:coal",0)
        s.expect_slot(pos,2,"minecraft:iron_ingot",0)
        result[label]={"old_burn_ticks":before,"new_burn_ticks":0,"free_outputs":0}
    return result


CASES={"timings":timings,"economics":economics,"augments":augments,"upgrades":upgrades,
       "menu":menu,"hoppers":hoppers,"blocked":blocked,"batch_capacity":batch_capacity,
       "remainders":remainders,"augment_swap":augment_swap,"persistence":persistence}

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--game",type=Path,required=True)
    p.add_argument("--ctl",type=Path,required=True)
    p.add_argument("--resources",type=Path,required=True)
    p.add_argument("--content",type=Path,default=ROOT/".build/migration-v3/mod")
    p.add_argument("--run-dir",type=Path,required=True)
    p.add_argument("--windowed",action="store_true",help="Start a rendered game; required for menu clicks")
    p.add_argument("--offscreen",action="store_true",help="Render without a visible window (requires --windowed)")
    p.add_argument("--keep-going",action="store_true",help="Record failures and try the remaining independent cases")
    p.add_argument("--only",default=",".join(CASES))
    args=p.parse_args()
    if args.offscreen and not args.windowed:
        p.error("--offscreen requires --windowed")
    for name in ("game","ctl","resources","content"):
        setattr(args,name,getattr(args,name).resolve(strict=True))
    if args.run_dir.exists(): p.error("--run-dir must not already exist")
    if args.run_dir.resolve().is_relative_to(args.resources): p.error("run directory must be outside resources")
    selected=args.only.split(",")
    if not all(name in CASES for name in selected): p.error("unknown case in --only")
    session=Session(args)
    try:
        session.start()
        for name in selected:
            if name=="menu" and not args.windowed:
                session.results[name]={"status":"skipped","reason":"click-slot requires --windowed"}
            else:
                try:
                    session.run_case(name,CASES[name])
                except Exception:
                    if not args.keep_going:
                        raise
                    # Only proceed if the disposable game still answers and
                    # we can return it to a known frozen state.
                    session.call("close")
                    session.command("tick freeze")
    finally:
        try: session.stop()
        finally: session.write()
    passed=sum(result["status"]=="passed" for result in session.results.values())
    skipped=sum(result["status"]=="skipped" for result in session.results.values())
    failed=sum(result["status"]=="failed" for result in session.results.values())
    print(f"Passed {passed} cases; failed {failed}; skipped {skipped}: {session.run/'results.json'}")
    if failed:
        raise SystemExit(1)

if __name__=="__main__":
    main()
