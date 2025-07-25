import subprocess
import shutil
import sys
import argparse

# build params (to be combined later)
features = "\"webgpu"
generic = " --filehash false";
default_build = features + generic

parser = argparse.ArgumentParser(prog='SkelForm Web Builder', description='Build script for SkelForm\'s web (WASM) version.', epilog="Default build command:\ntrunk build " + default_build)

# arguments
parser.add_argument('-s', '--serve', action='store_true', help="automatically run localhost:8000 after build")
parser.add_argument('-r', '--release', action='store_true', help="build for release/production")
parser.add_argument('-m', '--mobile', action='store_true', help="build for mobile")
parser.add_argument('-d', '--debug', action='store_true', help="build with debug flag. Ignored if --release is present")
parser.add_argument('-wgl', '--webgl', action='store_true', help="use webgl instead of webgpu")

args = parser.parse_args()

# add default release config, but only if not building for mobile
if args.release and not args.mobile:
    generic += " --release --public-url=/skelform_web"

if args.webgl:
    features = "\"webgl"

if args.mobile:
    features += " mobile"
if args.debug and not args.release:
    features += " debug"

features += "\""

build_command = "trunk build --features " + features + generic
print("\nBuild command:\n" + build_command + "\n")

# build /dist via trunk
subprocess.run(build_command, shell=True)

# copy assets over to /dist
shutil.copy("anim_icons.png", "dist/anim_icons.png")
shutil.copy("skf_icon.ico", "dist/favicon.ico")

if args.serve:
    # automatically serve via python http
    subprocess.run("python3 -m http.server 8000 --directory dist".split())
