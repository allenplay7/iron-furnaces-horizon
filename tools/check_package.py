"""Validate a release archive with the pinned Horizon package reader."""
from pathlib import Path
import argparse
import json
import os
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    args = parser.parse_args()
    source = args.source.resolve(strict=True)
    project = ROOT / '.build/migration-v3/package-check'
    (project / 'src').mkdir(parents=True, exist_ok=True)
    dependency = json.dumps((source / 'crates/horizon-mod-package').as_posix())
    (project / 'Cargo.toml').write_text(
        '[package]\nname="iron-furnaces-package-check"\nversion="0.1.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\nhorizon-mod-package={path=' + dependency + '}\n')
    (project / 'src/main.rs').write_text('''fn main() {
    let path = std::env::args().nth(1).expect("package path");
    match horizon_mod_package::Package::read(std::path::Path::new(&path)) {
        Ok(_) => println!("Pinned Horizon package reader accepted the release archive"),
        Err(error) => { eprintln!("Package rejected: {error}"); std::process::exit(1); }
    }
}
''')
    shutil.copyfile(source / 'Cargo.lock', project / 'Cargo.lock')
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
        raise SystemExit('Install the Rust version pinned by Horizon.')
    version = tomllib.loads((ROOT / 'native-v3/horizon-mod.toml').read_text())['version']
    archive = ROOT / 'dist' / f'ironfurnaces-{version}.hmod'
    subprocess.run([cargo, 'run', '--manifest-path', str(project / 'Cargo.toml'), '--', str(archive)], env=env, check=True)


if __name__ == '__main__':
    main()
