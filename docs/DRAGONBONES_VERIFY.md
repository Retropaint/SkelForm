# Verifying the DragonBones exporter

The exporter (`src/dragonbones_export.rs`, see `DRAGONBONES_EXPORT.md`) is checked
by loading its output in the **upstream DragonBonesCPP runtime** and comparing the
pose it computes, frame by frame, against the pose SkelForm computes for the same
rig. DragonBonesCPP is the runtime Inkternity embeds, so a match here means
Inkternity will play the rig the way SkelForm does.

Everything lives in `tools/dragonbones_verify/`:

| file | what it does |
|---|---|
| `dbharness.cpp` | Headless DragonBonesCPP program with no renderer and no image loading. Loads `<name>_ske.json` + `<name>_tex.json` and dumps bone matrices, slot vertices, display index, draw order and color. It does this for the setup pose and every frame of every animation |
| `build.bat` | Builds `build/dbharness.exe` with MSVC from a DragonBonesCPP checkout |
| `compare.py` | Diffs a harness dump against SkelForm's reference dump and prints max/mean error per category |
| `verify.py` | Runs everything: the exporter tests (which write the files), then the harness and the comparison for each rig |

`build/` is git-ignored.

## Setup (once)

Requirements: Windows, Visual Studio 2022 (or Build Tools) with the C++ toolset,
Python 3, and the Rust toolchain the project already uses.

```sh
git clone --depth 1 https://github.com/DragonBones/DragonBonesCPP.git <somewhere>
tools\dragonbones_verify\build.bat <somewhere>\DragonBonesCPP
```

You can also set the `DRAGONBONES_CPP` env var and run `build.bat` with no argument.
A line saying `'vswhere.exe' is not recognized` comes from Visual Studio's
`vcvars64.bat`. It is harmless.

## Run

```sh
python tools/dragonbones_verify/verify.py [out_dir]
```

`out_dir` defaults to `tools/dragonbones_verify/build/out`. The script does the following:

1. Runs `cargo test --test dragonbones_export` with `DB_OUT_DIR=out_dir`.
   - The tests export three rigs: a synthetic rig plus `samples/_skellington.skf`
     and `samples/_skellina.skf`.
   - Each rig gets `<name>_ske.json`, `<name>_tex.json` and `<name>_tex.png`.
   - Each rig also gets `<name>_ref.json`: SkelForm's own world-space pose for the
     setup and every frame, already converted to DragonBones' Y-down space.
2. Runs `dbharness <name>_ske.json <name>_tex.json <name>_db.json` on each rig.
3. Runs `compare.py <name>_ref.json <name>_db.json`.

Harness exit codes:
- 0 = ok
- 1 = bad input
- 2 = usage
- 3 = runtime assert
- 4 = crash

On a failure it prints the stage, e.g. `animation "Walk" frame 12`.

To check one of your own rigs, add a test in `tests/dragonbones_export.rs` that
loads it with `load_skf` and calls `export(&arm, "<name>")`, then rerun `verify.py`.

## Reading the output

```
bone pos               max=0.0098 mean=0.0004 n=8723  worst: Run @48 6_2
bone linear            max=0.0000 ...
draw order mismatch    max=0.0000 ...
visibility mismatch    max=0.0000 ...
verts                  max=0.0134 ...
color                  max=0.0000 ...
```

- **bone pos**: distance in px between the bone world positions.
- **bone linear**: largest difference in the rotation/scale part of the bone
  matrix (`a b c d`).
- **verts**: largest distance in px between a slot's world-space vertices. Points
  are matched nearest-first, because image quads can list corners in a different
  order.
- **visibility / draw order**: 1 on any frame where a slot is shown in one
  runtime and hidden in the other, or where the slot order differs.
- **color**: largest difference in the tint multipliers (0..1).
- **worst** names the animation, frame and bone/slot of the largest error.

Expected results as of 2026-09-28:

| rig | bone pos | verts | visibility / draw order / color |
|---|---|---|---|
| synthetic | 0.14 px | 0.33 px | exact (color ≤ 0.01, from integer %) |
| skellington | 0.01 px | 0.01 px | exact |
| skellina | 3.1 px | 0.57 px | exact |

The known, accepted differences:
- **Sub-pixel error on eased segments.** DragonBones samples a `curve` into
  `frames+1` points and interpolates linearly between them.
- **Skellina's 3.1 px.** Some of its keys have legacy all-zero Bézier handles.
  Mathematically that curve is linear, which is what gets exported. SkelForm's
  5-step Newton solver doesn't fully converge on it near t=0, so SkelForm itself is
  the one that's slightly off.

Anything past a few px, or any visibility/draw-order mismatch, is a regression.

## Scope and limits

- The harness checks geometry and slot state. It does not rasterize, so texture
  atlas **content** (the PNG) isn't checked, only SubTexture names and regions.
- Image corners are computed the way DragonBones' renderers place a sprite:
  `slot.globalTransformMatrix × region corners`, offset by the slot's pivot
  (see the comment in `dbharness.cpp`). Mesh vertices follow
  `CCSlot::_updateMesh`, without the cocos Y flip.
- The harness pins `DragonBones::yDown = true`, so all numbers stay in the
  JSON's native space.
- Windows/MSVC only. The C++ is portable, but only `build.bat` is provided.
