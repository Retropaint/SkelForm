"""Export the test rigs, load them in DragonBonesCPP, and diff against SkelForm's own poses.

usage: python tools/dragonbones_verify/verify.py [out_dir]

Needs tools/dragonbones_verify/build/dbharness.exe (see build.bat).
See docs/DRAGONBONES_VERIFY.md.
"""
import glob
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
HARNESS = os.path.join(HERE, "build", "dbharness.exe" if os.name == "nt" else "dbharness")


def main():
    out = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "build", "out"))
    os.makedirs(out, exist_ok=True)
    if not os.path.exists(HARNESS):
        sys.exit(f"missing {HARNESS}; run build.bat first")

    # the exporter tests write <name>_ske/_tex json, _tex.png and <name>_ref.json when DB_OUT_DIR is set
    env = dict(os.environ, DB_OUT_DIR=out)
    subprocess.run(["cargo", "test", "--test", "dragonbones_export"], cwd=REPO, env=env, check=True)

    failed = False
    for ske in sorted(glob.glob(os.path.join(out, "*_ske.json"))):
        name = os.path.basename(ske)[: -len("_ske.json")]
        tex = os.path.join(out, f"{name}_tex.json")
        ref = os.path.join(out, f"{name}_ref.json")
        dump = os.path.join(out, f"{name}_db.json")
        print(f"\n=== {name}")
        run = subprocess.run([HARNESS, ske, tex, dump])
        if run.returncode != 0:
            print(f"dbharness failed with exit code {run.returncode}")
            failed = True
            continue
        if os.path.exists(ref):
            subprocess.run([sys.executable, os.path.join(HERE, "compare.py"), ref, dump], check=True)

    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
