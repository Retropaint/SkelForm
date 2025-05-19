import subprocess
import shutil
import sys

# build /dist via trunk
subprocess.run("trunk build --features webgl --filehash false".split()) 

# copy assets over to /dist
shutil.copy("anim_icons.png", "dist/anim_icons.png")

if len(sys.argv) > 1 and sys.argv[1] == "--serve":
    # automatically serve via python http
    subprocess.run("python3 -m http.server 8000".split())
