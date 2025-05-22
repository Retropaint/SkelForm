import subprocess
import shutil
import sys
import argparse

default_build = "--features webgpu --filehash false";

parser = argparse.ArgumentParser(prog='SkelForm Web Builder', description='Build script for SkelForm\'s web (WASM) version.', epilog="Default build command:\ntrunk build " + default_build)

# arguments
parser.add_argument('-s', '--serve', action='store_true', help="(bool) Automatically run localhost:8000 after build.")
parser.add_argument('-b', '--build', default="", help="(string) Will be appended to trunk build. ex: --build \" --release\"")
parser.add_argument('-nd', '--no-default', action='store_true', help="(string) Don't use default config. Can be combined with --build for fully custom builds.")
parser.add_argument('-r', '--release', action='store_true', help="(bool) Use default release config.")

args = parser.parse_args()

# use default config if appropriate
if args.build == "" or not args.no_default:
    args.build = default_build + args.build

if args.release:
    args.build += " --release --public-url=/skelform_web"

build_command = "trunk build " + args.build.strip()
print("\nBuild command:\n" + build_command + "\n")

# build /dist via trunk
subprocess.run(build_command.split()) 

# copy assets over to /dist
shutil.copy("anim_icons.png", "dist/anim_icons.png")

if args.serve:
    # automatically serve via python http
    subprocess.run("python3 -m http.server 8000 --directory dist".split())
