# Verifying the DragonBones importer

The importer (`src/dragonbones_import.rs`, see `DRAGONBONES_IMPORT.md`) is checked
against the **upstream DragonBonesCPP runtime**, the reference implementation of
the format. Each rig is imported into SkelForm, the *original* files are played in
DragonBonesCPP, and the two poses are compared frame by frame. A match means an
imported rig plays in SkelForm the way it did in DragonBones.

Everything lives in `tools/dragonbones_verify/` and `tests/dragonbones_import.rs`:

| file | what it does |
|---|---|
| `tools/dragonbones_verify/dbharness.cpp` | Headless DragonBonesCPP program with no renderer and no image loading. Loads `<name>_ske.json` + `<name>_tex.json` and dumps bone matrices, slot vertices, display index, draw order and color. It does this for the setup pose and every frame of every animation |
| `tools/dragonbones_verify/build.bat` | Builds `build/dbharness.exe` with MSVC from a DragonBonesCPP checkout |
| `tests/dragonbones_import.rs` | Imports each rig, runs the harness on the original files, and compares |

`tools/dragonbones_verify/build/` is git-ignored. The tests are `#[ignore]`d, so a
plain `cargo test` doesn't need any of this.

## Setup (once)

Requirements: Windows, Visual Studio 2022 (or Build Tools) with the C++ toolset,
and the Rust toolchain the project already uses.

```sh
git clone --depth 1 https://github.com/DragonBones/DragonBonesCPP.git <somewhere>
tools\dragonbones_verify\build.bat <somewhere>\DragonBonesCPP
```

You can also set the `DRAGONBONES_CPP` env var and run `build.bat` with no argument.
A line saying `'vswhere.exe' is not recognized` comes from Visual Studio's
`vcvars64.bat`. It is harmless.

Harness exit codes: 0 = ok, 1 = bad input, 2 = usage, 3 = runtime assert,
4 = crash. On a failure it prints the stage, e.g. `animation "Walk" frame 12`.

## Run

```sh
set DB_SAMPLES=<folder of *_ske.json rigs, e.g. DragonBonesCPP/Cocos2DX_3.x/Demos/Resources>
cargo test --test dragonbones_import -- --ignored --nocapture
```

`DB_SAMPLES` is searched recursively; each `<name>_ske.json` needs its
`<name>_tex.json` (and `.png`) next to it. Harness dumps go to
`%TEMP%/skelform_db_import`.

There are three tests:
- `import_dragonbones_samples`: the pose comparison (below).
- `import_dragonbones_warnings`: every import warning per rig.
- `import_dragonbones_editor_warnings`: SkelForm's own rig warnings (the ⚠ list)
  on each imported rig.

## Reading the output

One line per rig:

```
            mecha_1406:  17 bones  308 frames ik | rot   0.06° (hit@1 thigh_1_r) | bone   0.181 (skill_01@5 shouder_l) | verts   0.897 px (death@17 chest) | vis 0 | warn 1
```

- **ik**: the rig has IK constraints.
- **rot**: largest bone rotation difference, in degrees.
- **bone**: largest distance in px between bone world positions.
- **verts**: largest distance in px between a slot's world-space vertices. Points
  are matched nearest-first, because image quads can list corners in a different
  order. Trimmed textures are compared by containment, because SkelForm pads them
  back to their frame.
- **vis**: frames where a slot is shown in one runtime and hidden in the other.
- **warn**: number of import warnings; the first few are listed below the line.
- The parentheses name the animation, frame and bone/slot of the worst error.

What's compared:
- Every frame of every animation, except the last one (the loop point).
- The setup pose, except for rigs with IK: SkelForm shows the setup pose without
  IK (`DRAGONBONES_IMPORT.md` §0).

Expected results as of 2026-09-29, on DragonBonesCPP's 43 sample rigs
(`Cocos2DX_3.x/Demos/Resources`):
- **40 rigs** within ~1 px, most under 0.1 px. These cover IK, weighted meshes,
  display swaps, and rotated/trimmed atlases.
- **Known exceptions:**
  - `you_xin/body` (listed as `body`): 20 px on one face mesh. It uses FFD (mesh deform) animation,
    which is unsupported and warned on import.
  - `mecha_1004d`: 3.8 px. A rotated display sits under a non-uniformly scaled
    bone, which is skew and can't be represented exactly.
  - `mecha_2903`: 1.06 px, from sub-degree skew in the file (editor rounding).

Anything past a few px on another rig, or any visibility mismatch, is a
regression.

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
