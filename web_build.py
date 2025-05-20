import subprocess
import shutil
import sys
import argparse

parser = argparse.ArgumentParser(prog='SkelForm Web Builder', description='Build script for SkelForm\'s web (WASM) version.')

# arguments
parser.add_argument('-s', '--serve', action='store_true', help="(bool) Automatically run localhost:8000 after build.")
parser.add_argument('-e', '--extra', default="", help="(string) Will be appended to trunk build. Must be enclosed in \"\".")

args = parser.parse_args()

# use a default config if no extras were provided
if args.extra == "":
    args.extra = "--features webgpu --filehash false" 

# build /dist via trunk
subprocess.run(("trunk build " + args.extra).split()) 

# copy assets over to /dist
shutil.copy("anim_icons.png", "dist/anim_icons.png")

if args.serve:
    # automatically serve via python http
    subprocess.run("python3 -m http.server 8000 --directory dist".split())
