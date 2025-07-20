# Script for building a complete release distribution.
#
# The full distribution requires, but is not limited to:
# - main binary (release version)
# - user documentation (built/distributed, not source)
# - source code & assets

import subprocess
import os
import platform
import shutil

RED = "\033[31m"
RESET = "\033[0m"


def require_docs(header, doc_name):
    print(f"{RED}!! {header} DOCUMENTATION REQUIRED !!{RESET}")
    print(f"1. Build it - https://github.com/retropaint/skelform_{doc_name}")
    print(f"2. Move `book` dir here and rename to '{doc_name}'")
    print("")


# Require user & dev docs before building
can_build = True
if not os.path.exists("user_docs"):
    require_docs("USER", "user_docs")
    can_build = False
if not os.path.exists("dev_docs"):
    require_docs("DEV", "dev_docs")
    can_build = False
if not can_build:
    exit()

binExt = ".exe" if platform.system == "Windows" else ""

platform_name = ""
match platform.system():
    case "Windows":
        platform_name = "windows"
    case "Darwin":
        platform_name = "mac"
    case "Linux":
        platform_name = "linux"

version = ""
with open("../cargo.toml", "r") as file:
    for line in file.readlines():
        if "version" in line.strip():
            version = line.strip().split('"')[1]
            break

dirname = "skelform_" + platform_name + "_v" + version

if os.path.exists(dirname):
    shutil.rmtree(dirname)
os.mkdir(dirname)

subprocess.run("cargo build --release", shell=True)
shutil.copy("../target/release/SkelForm" + binExt, "./" + dirname)
shutil.copytree("./user_docs", "./" + dirname + "/user_docs")
shutil.copytree("./dev_docs", "./" + dirname + "/dev_docs")

# Source code distribution

source = dirname + "/source"
os.mkdir("./" + source)
shutil.copy("../Cargo.toml", "./" + source)
shutil.copy("./release.py", "./" + source)
shutil.copy("../web_build.py", "./" + source)
shutil.copy("../anim_icons.png", "./" + source)
shutil.copy("../readme.md", "./" + source)
shutil.copytree("../src", "./" + source + "/src")

# Platform-specific distribution

if platform.system() == "Darwin":
    bin_path = "./mac_wrapper/SkelForm.app/Contents/MacOS/"
    if os.path.exists(bin_path):
        shutil.rmtree(bin_path)
    shutil.copytree(dirname, bin_path)
