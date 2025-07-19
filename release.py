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

with open("cargo.toml", "r") as file:
    all_lines = file.readlines()
    for line in all_lines:
        if "version" in line.strip():
            version = line.strip().split('"')[1]
            break

dirname = "skelform_" + platform_name + "_v" + version

can_build = True
RED = "\033[31m"
RESET = "\033[0m"
# Very politely ask for the user docs distribution
if not os.path.exists("user_docs"):
    print(f"{RED}!! USER DOCUMENTATION REQUIRED !!{RESET}")
    print("1. Build it - https://github.com/Retropaint/skelform_user_docs")
    print("2. Move `book` dir here and rename to 'user_docs'")
    print("")
    can_build = False

# Very politely ask for the dev docs distribution
if not os.path.exists("dev_docs"):
    print(f"{RED}!! DEVELOPER DOCUMENTATION REQUIRED !!{RESET}")
    print("1. Build it - https://github.com/Retropaint/skelform_dev_docs")
    print("2. Move `book` dir here and rename to 'dev_docs'")
    can_build = False

if not can_build:
    exit()

# remove dist folder if it already exists
if os.path.exists(dirname):
    shutil.rmtree(dirname)

# create dist folder, where everything will go
os.mkdir(dirname)

# create binary
build_command = "cargo build --release"
subprocess.run(build_command, shell=True)

# move binary to dist
shutil.copy("./target/release/SkelForm" + binExt, "./" + dirname)

shutil.copytree("./user_docs", "./" + dirname + "/user_docs")
shutil.copytree("./dev_docs", "./" + dirname + "/dev_docs")
shutil.copy("./readme.txt", "./" + dirname)

# Source code distribution

# create source code folder
source = dirname + "/source"
os.mkdir("./" + source)

# copy relevant stuff
shutil.copy("Cargo.toml", "./" + source)
shutil.copy("release.py", "./" + source)
shutil.copy("web_build.py", "./" + source)
shutil.copy("anim_icons.png", "./" + source)
shutil.copy("readme.md", "./" + source)
shutil.copytree("src", "./" + source + "/src")
