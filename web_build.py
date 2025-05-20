import subprocess
import shutil
import sys
import argparse

parser = argparse.ArgumentParser(prog='SkelForm Web Builder', description='Build script for SkelForm\'s web (WASM) version.')
parser.add_argument('-s', '--serve', action='store_true')
parser.add_argument('-r', '--release', action='store_true')
args = parser.parse_args()

# build /dist via trunk
extra = ""
if args.release:
    extra = "--public-url /skelform_web"
subprocess.run(("trunk build --features webgl --filehash false " + extra).split()) 

# copy assets over to /dist
shutil.copy("anim_icons.png", "dist/anim_icons.png")

if args.serve:
    # automatically serve via python http
    subprocess.run("python3 -m http.server 8000 --directory dist".split())
