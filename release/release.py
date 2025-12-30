# Script for building a complete release distribution.
#
# The full distribution requires, but is not limited to:
# - main binary (release version)
# - user documentation (built/distributed, not source)

import subprocess
import os
import platform
import shutil
import argparse
import zipfile

RED = "\033[31m"
BLUE = "\033[34m"
CYAN = "\033[36m"
RESET = "\033[0m"

# yapf: disable
parser = argparse.ArgumentParser(prog="SkelForm Release Builder", description="Build script for SkelForm release distributions.", formatter_class=argparse.RawTextHelpFormatter)
parser.add_argument("-v", "--verbose", action="store_true", help="Print output of everything")
parser.add_argument("-dmg", "--dmg", action="store_true", help="Attempt to create Mac dmg (requires create-dmg)")
parser.add_argument("-dbg", "--debug", action="store_true", help="Create debug build")
parser.add_argument("-nd", "--nodocs", action="store_true", help="Skip user docs & dev docs")
args = parser.parse_args()

stdout = "" if args.verbose else " &> /dev/null"

if not args.nodocs:
    shutil.rmtree("skelform_dev_docs", ignore_errors=True)
    shutil.rmtree("skelform_user_docs", ignore_errors=True)
    shutil.rmtree("user_docs", ignore_errors=True)
    shutil.rmtree("dev_docs", ignore_errors=True)
    subprocess.run("cargo install mdbook@0.5.1", shell=True)
    subprocess.run("git clone https://github.com/Retropaint/skelform_dev_docs", shell=True)
    subprocess.run("git clone https://github.com/Retropaint/skelform_user_docs", shell=True)
    subprocess.run("mdbook build skelform_dev_docs", shell=True)
    subprocess.run("mdbook build skelform_user_docs", shell=True)
    shutil.copytree("skelform_dev_docs/book", "./dev-docs", dirs_exist_ok = True)
    shutil.copytree("skelform_user_docs/book", "./user-docs", dirs_exist_ok = True)

# Require create-dmg on mac
if platform.system() == "Darwin" and not shutil.which("create-dmg") and args.dmg:
    print(f">>> {RED}!! create-dmg REQUIRED !!{RESET}")
    print(">>> Install create-dmg - https://github.com/create-dmg/create-dmg")
    exit()

binExt = ".exe" if platform.system() == "Windows" else ""

platform_name = ""
match platform.system():
    case "Windows":
        platform_name = "windows"
    case "Darwin":
        platform_name = "mac"
    case "Linux":
        platform_name = "linux"

version = ""
with open("../Cargo.toml", "r") as file:
    for line in file.readlines():
        if "version" in line.strip():
            version = line.strip().split('"')[1]
            break

dirname = "skelform_" + platform_name + "_v" + version

if os.path.exists(dirname):
    shutil.rmtree(dirname)
os.mkdir(dirname)

mode = "--release"
path = "release"
if args.debug:
    mode = ""
    path = "debug"

# yapf: disable
subprocess.run (f"cargo build {mode}", shell=True)
shutil.copy    (f"../target/{path}/SkelForm{binExt}", f"./{dirname}")
if not args.nodocs:
    shutil.copytree("./user-docs", f"./{dirname}/user-docs")
    shutil.copytree("./dev-docs",  f"./{dirname}/dev-docs")
shutil.copytree("../assets",      f"./{dirname}/assets")
shutil.copytree("../samples",     f"./{dirname}/samples")

# Platform-specific distribution

if platform.system() != "Darwin":
    shutil.make_archive(dirname, 'zip', ".", dirname)
else:
    print(">>> Preparing Mac app...")
    bin_path = "./SkelForm.app/Contents/MacOS/"
    if os.path.exists(bin_path):
        shutil.rmtree(bin_path)
    shutil.copytree(dirname, bin_path)

    # sign the app in any way, so the OS doesn't show 'this app is damaged'
    subprocess.run("codesign --force --deep --sign - SkelForm.app", shell=True)    

    shutil.make_archive("SkelForm.app", "zip", ".", "SkelForm.app")
    
    if not args.dmg:
        print(f">>> Mac release complete. Please look for {BLUE}SkelForm.app{RESET}.")
        exit()
    print(
        ">>> Preparing Mac dmg...\n    The dmg will instantly open, but you should still wait."
    )
    subprocess.run("./create-dmg.sh" + stdout, shell=True)
    print(f">>> Mac release complete. Please look for {BLUE}SkelForm.dmg{RESET}.")
