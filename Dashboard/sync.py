#!/usr/bin/env python3
"""Build and copy the standalone archive runtime; never copies measurement data."""
from pathlib import Path
import argparse,shutil,subprocess
here=Path(__file__).resolve().parent
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--build',action='store_true');args=parser.parse_args()
if args.build:subprocess.run(['npm','run','build'],cwd=here/'frontend',check=True)
target=here.parents[1]/'Messungen/GUI';target.mkdir(exist_ok=True)
shutil.copy2(here/'server.py',target/'server.py')
shutil.copytree(here/'dist',target/'dist',dirs_exist_ok=True)
# Remove obsolete hashed bundles after rebuilding, retaining only the current build.
current={p.relative_to(here/'dist') for p in (here/'dist').rglob('*') if p.is_file()}
for p in (target/'dist').rglob('*'):
    if p.is_file() and p.relative_to(target/'dist') not in current:p.unlink()
print('Standalone-GUI synchronisiert:',target)
