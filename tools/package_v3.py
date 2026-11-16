"""Package the unified native mod for Horizon's prebuilt-game API (Python 3.11+)."""
from pathlib import Path
import hashlib
import json
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def package():
    native = ROOT / 'native-v3'
    content = ROOT / 'content/ironfurnaces'
    manifest = tomllib.loads((native / 'horizon-mod.toml').read_text())
    entries = {}
    for folder in ('assets', 'data'):
        for path in (content / folder).rglob('*'):
            if path.is_file():
                entries[path.relative_to(content).as_posix()] = path.read_bytes()
    for path in (native / 'src').rglob('*.rs'):
        entries[path.relative_to(native).as_posix()] = path.read_bytes()
    for name in ('Cargo.toml', 'Cargo.lock', 'horizon-mod.toml'):
        entries[name] = (native / name).read_bytes()
    for name in ('LICENSE', 'NOTICE', 'MODIFICATIONS.md'):
        entries[name] = (ROOT / name).read_bytes()
    stage = ROOT / '.build/migration-v3/mod'
    stage.mkdir(parents=True, exist_ok=True)
    # Only our generated stage may be replaced; never touch a user's installed mod.
    for path in stage.rglob('*'):
        if path.is_file() and path.relative_to(stage).as_posix() not in entries:
            path.unlink()
    for name, data in entries.items():
        path = stage / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    out = ROOT / 'dist' / f'ironfurnaces-{manifest["version"]}.hmod'
    out.parent.mkdir(exist_ok=True)
    # Stored entries avoid different zlib versions emitting different deflate
    # streams on Windows/Linux. The small source package stays reproducible.
    with zipfile.ZipFile(out, 'w', compression=zipfile.ZIP_STORED) as archive:
        for name, data in sorted(entries.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.create_system = 3  # Stable Unix metadata on every build platform.
            info.compress_type = zipfile.ZIP_STORED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, data)
    digest = hashlib.sha256(out.read_bytes()).hexdigest()
    out.with_suffix('.hmod.sha256').write_text(f'{digest}  {out.name}\n', encoding='utf-8', newline='\n')
    print(json.dumps({'package': str(out), 'sha256': digest, 'files': len(entries), 'stage': str(stage)}, indent=2))
    return out


if __name__ == '__main__':
    package()
