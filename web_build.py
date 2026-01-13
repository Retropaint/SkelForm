import subprocess
import shutil
import sys
import argparse

RED = "\033[31m"
RESET = "\033[0m"

if not shutil.which("trunk"):
    print(f">>> {RED}!! trunk REQUIRED !!{RESET}")
    print(">>> Install trunk - https://trunkrs.dev/")
    print("")
    exit()

# build params (to be combined later)
features = '"'
generic = " --filehash false"
default_build = features + generic

parser = argparse.ArgumentParser(
    prog="SkelForm Web Builder",
    description="Build script for SkelForm's web (WASM) version.",
    epilog="Default build command:\ntrunk build " + default_build,
)

# yapf: disable
parser.add_argument("-s",   "--serve",   action="store_true", help="automatically run localhost:8000 after build",)
parser.add_argument("-r",   "--release", action="store_true", help="build for release/production")
parser.add_argument("-m",   "--mobile",  action="store_true", help="build for mobile")
parser.add_argument("-d",   "--debug",   action="store_true", help="build with debug flag. Ignored if --release is present",)
parser.add_argument("-u",   "--baseurl", action="store", help="Sets the base url. Overrides url from --release",)
parser.add_argument("-wg",   "--webgpu", action="store", help="Builds with webgpu feature instead of webgl",)

args = parser.parse_args()

if args.release and not args.mobile:
    generic += " --release"
    if not args.serve and not args.baseurl:
        generic += " --public-url=/editor"
    elif args.baseurl:
        generic += " --public-url=/" + args.baseurl
if args.mobile:
    features += " mobile"
if args.debug and not args.release:
    features += " debug"
if args.webgpu:
    features += "webgpu"
else:
    features += "webgl"

features += '"'

build_command = "trunk build --features " + features + generic
print("\nBuild command:\n" + build_command + "\n")

# build /dist via trunk
subprocess.run(build_command, shell=True)

# copy assets over to /dist
shutil.copy("assets/skf_icon.ico", "dist/favicon.ico")
shutil.copy("samples/_skellington.skf", "dist/_skellington.skf")
shutil.copy("samples/_skellina.skf", "dist/_skellina.skf")

if args.serve:
    # automatically serve via python http
    subprocess.run("python3 -m http.server 8000 --directory dist".split())
