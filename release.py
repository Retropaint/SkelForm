# Script for building a complete release distribution.
#
# The full distribution requires, but is not limited to:
# - main binary (release version)
# - user documentation (built/distributed, not source)
# - source code & assets

import subprocess
import os
import shutil

binExt = ".exe" if os.name == 'nt' else ""

# Very politely ask for the user docs distribution
if not os.path.exists("book"):
    print("User documentation required:")
    print("1. Build it - https://github.com/Retropaint/skelform_user_docs")
    print("2. Move `book` dir here")
    exit()

dirname = "release"

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

# copy user_docs to dist
shutil.copytree("./book", "./" + dirname + "/user_docs")

# Source code distribution

# create source code folder
source = dirname + "/source";
os.mkdir("./" + source)

# copy relevant stuff
shutil.copy("Cargo.toml", "./" + source)
shutil.copy("release.py", "./" + source)
shutil.copy("web_build.py", "./" + source)
shutil.copy("anim_icons.png", "./" + source)
shutil.copy("readme.md", "./" + source)
shutil.copytree("src", "./" + source + "/src")
