"""Check the new native adapter against an exact local Horizon source checkout."""
from pathlib import Path
import argparse
import os
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--test', action='store_true')
    args = parser.parse_args()
    source = args.source.resolve()
    dest = ROOT / '.build/migration-v3/check-project'
    dest.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / 'native-v3/src', dest / 'src', dirs_exist_ok=True)
    manifest = (ROOT / 'native-v3/Cargo.toml').read_text()
    manifest = manifest.replace('.horizon/sdk/api', source.as_posix())
    manifest += '\n[workspace]\n'
    (dest / 'Cargo.toml').write_text(manifest)
    # Seed resolution from the same dependency graph as Horizon's prebuilt SDK.
    # Cargo adds this standalone mod without floating the SDK's shared crates.
    shutil.copyfile(source / 'Cargo.lock', dest / 'Cargo.lock')
    env = dict(os.environ)
    if os.name == 'nt' and env.get('APPDATA'):
        launcher = Path(env['APPDATA']) / 'horizon-launcher'
        rust = launcher / 'rust/1.99.1-x86_64-pc-windows-gnu'
        if (rust / 'bin/cargo.exe').is_file():
            env['PATH'] = os.pathsep.join([str(rust / 'bin'), str(rust / 'mingw64/bin'), env['PATH']])
            env.setdefault('CARGO_HOME', str(launcher / 'cargo'))
    env['CARGO_TARGET_DIR'] = str(ROOT / '.build/migration-v3/api-check')
    env['CARGO_BUILD_JOBS'] = '1'
    cargo = shutil.which('cargo', path=env['PATH'])
    if not cargo:
        raise SystemExit('Install the Rust version pinned by Horizon, or its managed Windows toolchain.')
    subprocess.run([cargo, 'test' if args.test else 'check', '--manifest-path', str(dest / 'Cargo.toml')], env=env, check=True)

if __name__ == '__main__':
    main()
