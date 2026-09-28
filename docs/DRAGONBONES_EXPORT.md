# Add a DragonBones-JSON exporter to SkelForm

**Goal:** give SkelForm an **Export → DragonBones** action that writes a rig in the
DragonBones JSON format, so it round-trips with **Inkternity** (an infinite-canvas
painting app). This is the return leg of an existing pipeline:

```
Inkternity  --(Export PSD, layered)-->  SkelForm  --(rig + animate)-->  ??? 
   ^                                                                      |
   +----------------(Import 2D Skeletal Rig: DragonBones)-----------------+
```

Inkternity already **imports** DragonBones rigs as live, animated on-canvas
components (it embeds the MIT `DragonBonesCPP` runtime). It already **exports** a
layered PSD that SkelForm imports (PSD groups → bones). The only missing piece is
SkelForm emitting DragonBones JSON so the animated character can go back into
Inkternity. **That is this task.**

> **Licensing.** This exporter lives in SkelForm's own (GPL) repo — that's fine.
> Inkternity never links SkelForm code; it only reads the DragonBones files SkelForm
> writes. Do not copy Inkternity code into SkelForm or vice-versa. This repo is the
> fork `inviti8/SkelForm`; open a PR upstream to `Retropaint/SkelForm` when ready.

---

## 1. Deliverable

Three files (the DragonBones convention Inkternity consumes), sharing a base name:

| file | contents |
|------|----------|
| `<name>_ske.json` | skeleton: armature, bones, slots, skins (displays), animations |
| `<name>_tex.json` | texture atlas: `SubTexture` regions into the PNG |
| `<name>_tex.png`  | the packed atlas image |

Inkternity's importer lets the user pick `<name>_ske.json`; it then looks for the
two siblings **`<name>_tex.json`** and **`<name>_tex.png`** next to it. So:
- name all three with the same `<base>` and the `_ske` / `_tex` suffixes,
- set `_tex.json`'s `"imagePath"` to `"<base>_tex.png"`,
- offer to write them into one folder (a `.zip` is fine too, but the three loose
  files with matching names are what the importer expects on disk).

Inkternity parses **DragonBones 5.5 JSON** via `DragonBonesCPP`'s `JSONDataParser`
(there is also a binary `.dbbin` form — **do not** target that; JSON only).

---

## 2. Target format (authoritative reference)

Field shapes below are taken from a real rig Inkternity ships as its import
fixture (`mecha_1004d`). Coordinates are DragonBones' (see the fidelity notes in
§4 for conversions from SkelForm).

### 2.1 `<name>_ske.json`

```jsonc
{
  "frameRate": 24,                 // global fps
  "name": "character",             // dragonBonesData name
  "version": "5.5",
  "compatibleVersion": "5.5",
  "armature": [ { /* one Armature */ } ]
}
```

An **armature**:

```jsonc
{
  "type": "Armature",
  "frameRate": 24,
  "name": "character",             // referenced by importer as armature[0]
  "aabb": { "x": 0, "y": 0, "width": 0, "height": 0 },   // may be zeros
  "bone":  [ /* Bone[] */ ],
  "slot":  [ /* Slot[] */ ],
  "skin":  [ /* Skin[] */ ],       // at least one, name "" is fine (default skin)
  "animation": [ /* Animation[] */ ],
  "defaultActions": [ { "gotoAndPlay": "idle" } ]   // optional
}
```

**Bone** — hierarchy by `parent` *name* (root bone omits `parent`):

```jsonc
{ "name": "pelvis", "parent": "root",
  "transform": { "x": 0, "y": -120, "skX": 0, "skY": 0, "scX": 1, "scY": 1 } }
```
- `transform` is the bone's rest pose **relative to its parent**. `x`/`y` in px,
  `skX`/`skY` = rotation in **degrees** (skew; for a plain rotation skX==skY),
  `scX`/`scY` = scale. Omit `transform` for an identity bone.

**Slot** — one per textured bone; `parent` = the **bone name** it rides:

```jsonc
{ "name": "head", "parent": "head", "displayIndex": 0, "z": 5,
  "color": { "aM": 100, "rM": 100, "gM": 100, "bM": 100 } }   // color optional; *M = %
```
- `z` sets draw order (higher = in front). DragonBones has **bone → slot →
  display**; SkelForm has no slot concept, so **synthesize one slot per display-
  bearing bone** (§4).

**Skin** — maps each slot to its display(s):

```jsonc
{ "name": "", "slot": [
    { "name": "head",                       // = slot name
      "display": [
        { "name": "atlas/head",             // MUST equal a SubTexture name in _tex.json
          "type": "image",                  // "image" | "mesh" | "armature"
          "transform": { "x": 0, "y": 0, "scX": 1, "scY": 1 },  // display offset within the slot (pivot)
          "pivot": { "x": 0.5, "y": 0.5 } } // optional normalized pivot
      ] }
] }
```

**Mesh display** (when a SkelForm bone has an edited mesh + weights):

```jsonc
{ "name": "atlas/cloak", "type": "mesh",
  "width": 200, "height": 300,             // source region px
  "vertices": [x0,y0, x1,y1, ...],         // local-space, px
  "uvs": [u0,v0, u1,v1, ...],              // normalized 0..1, atlas-relative
  "triangles": [0,1,2, 2,3,0, ...],        // uint indices
  // weighted skin (FFD/skinning):
  "weights": [ /* per-vertex: boneCount, (boneIdx, weight)... */ ],
  "bonePose": [ /* per bound bone: boneIdx, a,b,c,d,tx,ty */ ],
  "slotPose": [ a,b,c,d,tx,ty ] }
```
> Meshes are the trickiest part. If it saves scope, ship **cutout first** (every
> display `type:"image"`) and gate meshes behind a follow-up — but SkelForm *does*
> support meshes + multi-bone weights, so a complete exporter should emit them.
> Study `DragonBonesCPP`'s `JSONDataParser::_parseMesh`/`_parseSkin` for the exact
> `weights`/`bonePose`/`slotPose` packing before implementing.

**Animation** — per-bone transform frames + per-slot display/color frames:

```jsonc
{ "name": "idle", "duration": 60, "playTimes": 0, "fadeInTime": 0,
  "bone": [
    { "name": "pelvis",
      "translateFrame": [ { "duration": 30, "tweenEasing": 0, "x": 0,  "y": 0 },
                          { "duration": 30, "tweenEasing": 0, "x": -2, "y": 0 },
                          { "duration": 0 } ],
      "rotateFrame":    [ { "duration": 30, "tweenEasing": 0, "rotate": 0 },
                          { "duration": 30, "tweenEasing": 0, "rotate": -1.5 },
                          { "duration": 0 } ],
      "scaleFrame":     [ /* {duration, tweenEasing, x, y} */ ] }
  ],
  "slot": [
    { "name": "head",
      "displayFrame": [ { "duration": 60, "value": 0 } ],           // display index (-1 = hidden)
      "colorFrame":   [ { "duration": 60, "value": {"aM":100} } ] }
  ]
}
```
- `duration` is **in frames**, and is the length of THAT frame (the gap until the
  next). The last frame commonly has `duration: 0`. The animation's top-level
  `duration` = sum of a track's frame durations = total frames.
- `playTimes`: `0` = loop, `1` = once, n = n times.
- `tweenEasing`: `0` = linear, `NaN`/absent = no tween (step/hold),
  `(0,1]`/`[-1,0)` = quad ease out/in. Arbitrary curves use a `"curve": [...]`
  sample array on the frame instead of `tweenEasing` (see §4 interpolation).
- `rotate` is **degrees**, relative to the bone's rest rotation.

### 2.2 `<name>_tex.json`

```jsonc
{ "name": "character",
  "imagePath": "character_tex.png",
  "width": 1024, "height": 1024,          // atlas image size
  "SubTexture": [
    { "name": "atlas/head", "x": 0, "y": 0, "width": 128, "height": 128 },
    // optional trim fields for packed-with-whitespace sprites:
    // "frameX": -4, "frameY": -2, "frameWidth": 136, "frameHeight": 132
  ] }
```
- Each `SubTexture.name` is what displays reference by `display.name`. **The names
  must match exactly** or that slot renders nothing.

### 2.3 Minimal sanity example

One bone, one image slot, one 1-frame idle → a static rig that imports and draws.
Build this first as a smoke test before tackling animation/mesh.

---

## 3. SkelForm source map (where to build)

All paths under `D:\repos\SkelForm`. Single Cargo crate (lib `skelform_lib` +
bin), egui/wgpu app, native + wasm targets. Save/export code is `#[cfg]`-split
between native (`rfd` file dialog → disk) and wasm (in-memory `Cursor` → JS); a
DragonBones exporter should do **both** or be gated native-only, matching the
existing exporters.

**Data model — `src/shared.rs`:**
- `Armature` (~L1298): `bones: Vec<Bone>`, `animations: Vec<Animation>`,
  `styles: Vec<Style>` (texture sets/skins), `tex_data: Vec<TextureData>`
  (decoded images, runtime-only). Sampler `Armature::animate(anim_idx, frame,
  og_bones)` (~L1415).
- `Bone` (~L1116): `id`, `name`, `parent_id` (flat list, hierarchy by parent id;
  children stored contiguously), `pos/scale: Vec2`, `rot: f32` **(radians)**,
  `zindex`, `pivot_pos/rot/scale`, `tex: String` (texture name), `vertices:
  Vec<Vertex>`, `indices: Vec<u32>`, `verts_edited: bool`, `binds:
  Vec<BoneBind>`, `init_pos/init_rot/init_scale` (rest pose). Many of these are
  `#[serde(skip)]` and migrated into `Visuals` on save.
- Mesh: `Vertex` (~L190, `pos/uv/init_pos`), weights `BoneBind`(~L1233)
  `{bone_id, verts: Vec<BoneBindVert>}`, `BoneBindVert`(~L1242) `{id, weight}`.
- `Animation` (~L1851): `name`, `fps: i32`, **flat** `keyframes: Vec<Keyframe>`.
  `Keyframe` (~L1955): `frame`, `bone_id`, `element: AnimElement`, `value: f32` /
  `value_str`, `start_handle/end_handle: Vec2` (**cubic Bézier** handles),
  `next_kf`, `handle_preset`. `AnimElement` (~L1997):
  `PositionX/Y, Rotation, ScaleX/Y, Zindex, Texture, Hidden, TintR/G/B/A,
  IkConstraint, ...` — **one scalar channel per keyframe**.
- Atlas region `Texture` (~L1822) `{name, ser_offset: Vec2I, ser_size: Vec2I,
  atlas_idx, ...}`, grouped under `Style` (~L1800) `{name, textures: Vec<Texture>}`.
  `TexAtlas` (~L1749) `{filename, size}`.

**Existing exporters to mirror — `src/utils.rs`, `src/lib.rs`:**
- `utils::create_tex_sheet(armature, edit_mode) -> (Vec<Vec<u8>>, Vec<i32>)`
  (~L422) — packs `tex_data` into square atlas PNG(s) with the vendored
  `max_rects` bin-packer and writes each `Texture.offset` back. **Reuse this to
  get `_tex.png` + the rects for `_tex.json`.**
- `utils::prepare_files(...)` (~L557) — the "serialize model" step: strips dead
  keyframes, optionally bakes IK to rotation frames, fills `next_kf`, and
  **re-indexes IDs→array positions**. For DragonBones you want **name-based**
  references, so run your exporter against the **pre-remap** armature (real
  ids/names), or reuse only its IK-bake + keyframe-cleanup helpers.
- `App::save` (`lib.rs` ~L1284) — native zip assembly (calls `create_tex_sheet`
  then `prepare_files`, writes files). Template for writing output.
- `utils::save_web` (~L148) — wasm path (`Cursor` + `zip`, bytes to JS).
- Other templates: spritesheet (`utils::render_spritesheets` ~L191),
  per-style PNG export (`App::check_export_style` `lib.rs` ~L927).

**Dispatch to wire into:**
1. `Saving` enum — `shared.rs` ~L2146 → add `DragonBones`.
2. `utils::open_save_dialog` match — `utils.rs` ~L92-102 → add `("json",
   "DragonBones")` (or a folder/zip).
3. Save loop — `lib.rs` ~L846-870 polls `shared.ui.saving`; add a branch calling
   your writer.
4. Export modal — `src/export_modal.rs` `draw()` (~L33) has Armature/Image/Video
   tabs (~L74-83); add a **4th "DragonBones" tab** (options: embed atlas, bake IK,
   cutout-only). Its Export button sets `Saving::DragonBones` + `open_save_dialog`.
5. File menu — `src/ui.rs` `top_bar_file` (~L1534) → sibling menu item; i18n key
   in `assets/i18n/en.json`.

**PSD import (context; already done)** — `src/file_reader.rs` `read_psd` (~L206):
PSD groups → bones, each group flattened to one texture; `$pivot`/`$ik_*` marker
layers configure pivot/IK. This is why the pipeline works today; your exporter is
the reverse trip.

**Suggested shape:** new `src/dragonbones_export.rs` with
`fn export(armature: &Armature, atlas_png: Vec<Vec<u8>>, atlas_sizes: &[i32],
opts) -> DbFiles` producing the three files; wire through the five dispatch points
above; reuse `create_tex_sheet` for the atlas; imitate `App::save` for writing.

---

## 4. Model mapping + fidelity gotchas

| SkelForm | DragonBones | notes |
|---|---|---|
| `Bone` (flat, `parent_id`) | `bone` (by `parent` **name**) | resolve id→name; root omits `parent` |
| `Bone.rot` **radians** | `transform.skX/skY`, `rotateFrame.rotate` **degrees** | `deg = rad * 180/PI` |
| `Bone.pos/scale` | `transform.x/y`, `scX/scY` | **Y-axis / pivot conventions differ** — verify sign of Y and rotation direction against the sample; expect to flip Y |
| `Bone.tex` / `Visuals.tex` (name) | slot + skin `display.name` | **synthesize one slot per textured bone** (DB needs bone→slot→display) |
| `Bone.zindex` | slot `z` / `Zindex` frames | draw order |
| mesh `Vertex`+`indices`+`binds` | `type:"mesh"` display (`vertices/uvs/triangles/weights/bonePose/slotPose`) | UVs already normalized → direct; weights → DB weighted skin |
| `Style` (texture set) | `skin` | one skin per style; the active/default style → skin name `""` |
| `Texture` (`ser_offset/ser_size`, int) | `_tex.json` `SubTexture` (`x/y/width/height`) | use the **int** ser_* fields |
| `Animation.fps` | armature/animation `frameRate` | |
| flat `(bone_id, element)` keyframes | per-bone `translateFrame/rotateFrame/scaleFrame` + per-slot `displayFrame/colorFrame` | **re-aggregate** channels into DB frames; interpolate missing components; `Texture`→displayFrame, `Tint*`→colorFrame, `Hidden`→displayFrame value -1 |
| Bézier `start_handle/end_handle` | frame `tweenEasing` or `curve` sample array | linear→`tweenEasing:0`; general cubic→bake to a `curve` sample array (DB samples the curve); `HandlePreset` Sine/Snap won't all have 1:1 constants |
| IK families (`InverseKinematics`) | (partial) | SkelForm already can **bake IK → rotation keyframes** on export (`export_bake_ik` in `prepare_files`); default to baking |
| `Physics` (spring/sway) | — | no clean DB target; **drop** (or bake into keyframes if feasible) |

**Frame-duration model:** SkelForm keyframes have an absolute `frame`; DragonBones
frames carry a **relative `duration`** (frames until the next). Convert by sorting
a channel's keyframes by `frame`, then `duration[i] = frame[i+1] - frame[i]`, with
a trailing `{ "duration": 0 }`. Total animation `duration` = last `frame`.

---

## 5. Acceptance / testing

1. **JSON validity:** `serde_json` round-trip your output; then confirm it parses
   in DragonBones. Inkternity ships a headless validator you can build against if
   you have that repo: `tools/dragonbones_render_spike.cpp` loads a rig and renders
   a frame to PNG. Otherwise use any DragonBones runtime (e.g. `pixi-dragonbones`
   in a scratch web page) to load `<name>_ske.json` + `_tex.json` + `_tex.png`.
2. **Static first:** export a one-bone, one-image, one-frame rig → it must load and
   draw the sprite at the right place (validates bone transform, slot/skin/display
   wiring, and atlas SubTexture names).
3. **Animation:** export a 2–3 keyframe idle (translate + rotate) → it must play
   and loop; check rotation direction and Y sign against SkelForm's own playback.
4. **Round-trip:** open the exported files in **Inkternity** (File ▸ Import ▸
   *Import 2D Skeletal Rig*, pick `<name>_ske.json`). Confirm it appears as a live
   rig, animates, and the clip list matches your animation names.
5. **Compare to the fixture:** Inkternity's `deps/dragonbones/sample/mecha_1004d/`
   is a known-good rig — diff your output's shape against it when unsure.

Start cutout-only (no meshes, linear tweens, baked IK) to get a full round-trip
green, then layer in meshes/weights and curve export.

---

## 6. Status / ownership

- Inkternity side is **shipped** (v0.14.0-rc17): import, playback, group→PSD
  export, atlas re-skin. See Inkternity's `docs/design/ANIMATED_IMPORTS.md`.
- This doc is the SkelForm-side handoff (DragonBones exporter = the remaining
  round-trip leg). Fork: `inviti8/SkelForm`; upstream: `Retropaint/SkelForm`.
