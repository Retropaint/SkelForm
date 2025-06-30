# Script for building a complete release distribution.
#
# The full distribution requires, but is not limited to:
# - main binary (release version)
# - user documentation (built/distributed, not source)
# - source code & assets

import subprocess
import os
import shutil

dirname = "SkelForm"

# remove dist folder if it already exists
if os.path.exists(dirname):
    shutil.rmtree(dirname)

# create dist folder, where everything will go
os.mkdir(dirname)

# create binary
build_command = "cargo build --release"
subprocess.run(build_command, shell=True)

# move binary to dist
shutil.move("./target/release/SkelForm", "./" + dirname)

# copy user_docs to dist
shutil.copytree("./user_docs", "./" + dirname + "/user_docs")

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
