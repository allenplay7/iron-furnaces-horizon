"""Build, package and validate the Iron Furnaces Horizon port (Python 3.11+)."""
from __future__ import annotations

import argparse
import filecmp
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REVISION = "9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"
MIXINS_REVISION = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
BUNDLE = ROOT / "native/mixins/ironfurnaces"
BUILD = ROOT / ".build"


def run(command, *, cwd=ROOT, env=None):
    print("+ " + subprocess.list2cmdline(list(map(str, command))), flush=True)
    subprocess.run(list(map(str, command)), cwd=cwd, env=env, check=True)


def environment():
    env = dict(os.environ)
    launcher = Path(env.get("APPDATA", str(Path.home() / ".local/share"))) / "horizon-launcher"
    toolchains = sorted((launcher / "rust").glob("*/bin/cargo.exe"))
    if toolchains and not shutil.which("cargo"):
        toolchain = toolchains[-1].parent.parent
        env["PATH"] = os.pathsep.join([str(toolchain / "bin"), str(toolchain / "mingw64/bin"), env.get("PATH", "")])
    if "CARGO_HOME" not in env and (launcher / "cargo/registry").is_dir():
        env["CARGO_HOME"] = str(launcher / "cargo")
    env["CARGO_TARGET_DIR"] = str(BUILD / "target")
    return env, launcher


def zip_files(destination, entries):
    destination.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in sorted(entries.items()):
            entry = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            entry.external_attr = 0o100644 << 16
            archive.writestr(entry, data)


def package():
    manifest = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text(encoding="utf-8"))
    mod = manifest["mod"]
    if manifest["compatibility"]["horizon_revision"] != REVISION:
        raise ValueError("Native manifest revision differs from tested revision")
    metadata = dict(schema=1, id=mod["id"], version=mod["version"], game="26.3", horizon=REVISION, mixins=MIXINS_REVISION)
    # Omit `hooks`: this adapter uses pinned game internals, not only the stable API.
    entries = {"horizon-native.json": (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode()}
    prefix = f"mixins/{mod['id']}/"
    for path in BUNDLE.rglob("*"):
        if path.is_file() and path.suffix in (".rs", ".toml"):
            entries[prefix + path.relative_to(BUNDLE).as_posix()] = path.read_bytes()
    entries[prefix + "LICENSE"] = (ROOT / "LICENSE").read_bytes()
    entries[prefix + "LICENSE-NOTICE"] = (ROOT / "NOTICE").read_bytes()
    native = ROOT / "dist" / f"ironfurnaces-{mod['version']}.hnmod"
    zip_files(native, entries)
    content_root = ROOT / "content/ironfurnaces"
    entries = {p.relative_to(content_root).as_posix(): p.read_bytes() for p in content_root.rglob("*") if p.is_file()}
    if "horizon-mod.toml" not in entries:
        raise ValueError("Generate content first: python tools/build_content.py")
    entries["LICENSE"] = (ROOT / "LICENSE").read_bytes()
    entries["NOTICE"] = (ROOT / "NOTICE").read_bytes()
    content = ROOT / "dist" / f"ironfurnaces-{mod['version']}.hmod"
    zip_files(content, entries)
    hashes = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in (content, native)}
    (ROOT / "dist/SHA256SUMS").write_text("".join(f"{digest}  {name}\n" for name, digest in hashes.items()), encoding="utf-8")
    print(json.dumps(hashes, indent=2))
    return content, native


def stage(args, launcher, env):
    source = Path(args.source).resolve() if args.source else launcher / "native/source" / REVISION
    if not (source / "Cargo.toml").is_file():
        raise FileNotFoundError("Supply --source with Horizon at " + REVISION)
    tool = Path(args.mixins).resolve() if args.mixins else launcher / "native/tools" / MIXINS_REVISION[:12] / "bin/horizon-mixins.exe"
    if not tool.is_file():
        executable = shutil.which("horizon-mixins", path=env["PATH"])
        if not executable:
            raise FileNotFoundError("Supply --mixins with the horizon-mixins executable")
        tool = Path(executable)
    out = BUILD / "stage"
    digest = hashlib.sha256(str(source).encode())
    for path in sorted(BUNDLE.rglob("*")):
        if path.is_file():
            digest.update(path.relative_to(BUNDLE).as_posix().encode())
            digest.update(path.read_bytes())
    stamp = BUILD / "stage-input.sha256"
    if out.is_dir() and stamp.is_file() and stamp.read_text() == digest.hexdigest():
        return out
    # Windows cannot atomically replace an existing mixin stage. Generate a
    # fresh tree, then copy changed files into the stable Cargo source path.
    # Keeping unchanged files/timestamps also preserves Cargo's build cache.
    fresh = Path(tempfile.mkdtemp(prefix="mixin-stage-", dir=BUILD))
    fresh.rmdir()
    run([tool, "apply", "--workspace", source, "--mods", ROOT / "native/mixins", "--out", fresh], env=env)
    out.mkdir(exist_ok=True)
    for path in fresh.rglob("*"):
        relative = path.relative_to(fresh)
        destination = out / relative
        if path.is_dir():
            destination.mkdir(parents=True, exist_ok=True)
        elif not destination.is_file() or not filecmp.cmp(path, destination, shallow=False):
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
    stamp.write_text(digest.hexdigest())
    if fresh.resolve().parent != BUILD.resolve() or not fresh.name.startswith("mixin-stage-"):
        raise ValueError("Refusing to clean a stage outside the build directory")
    shutil.rmtree(fresh)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["test", "package", "stage", "check", "build", "validate"])
    parser.add_argument("--source", help="Clean pinned Horizon source tree")
    parser.add_argument("--mixins", help="horizon-mixins executable")
    parser.add_argument("--offline", action="store_true", help="Use already cached Rust crates")
    args = parser.parse_args()
    env, launcher = environment()
    cargo = shutil.which("cargo", path=env["PATH"])
    BUILD.mkdir(exist_ok=True)
    if args.action == "package":
        package()
        return
    if args.action == "test":
        env["CARGO_TARGET_DIR"] = str(BUILD / "rules-target")
        if not cargo:
            raise FileNotFoundError("Install Rust or prepare Horizon native mods in the launcher first")
        run([cargo, "test", "--locked", *(["--offline"] if args.offline else [])], env=env)
        run([sys.executable, "-m", "unittest", "discover", "-s", "tools", "-p", "test_*.py"], env=env)
        return
    if args.action in ("stage", "check", "build"):
        staged = stage(args, launcher, env)
        if args.action == "stage":
            return
        if not cargo:
            raise FileNotFoundError("Rust cargo not found")
        action = "check" if args.action == "check" else "build"
        run([cargo, action, "--locked", "--profile", "fast", "-p", "horizon-client", *(["--offline"] if args.offline else [])], cwd=staged, env=env)
        return
    packages = package()
    executable = BUILD / "target/fast" / ("cargo-horizon-mod.exe" if os.name == "nt" else "cargo-horizon-mod")
    if not executable.exists():
        if not cargo:
            raise FileNotFoundError("Rust cargo not found")
        source = Path(args.source).resolve() if args.source else launcher / "native/source" / REVISION
        run([cargo, "build", "--locked", "--profile", "fast", "--manifest-path", source / "Cargo.toml", "-p", "cargo-horizon-mod", *(["--offline"] if args.offline else [])], env=env)
    for path in packages:
        run([executable, "check", path, "--json"], env=env)


if __name__ == "__main__":
    main()
