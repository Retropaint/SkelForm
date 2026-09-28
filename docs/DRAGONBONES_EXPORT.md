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

### 1.1 Decisions (2026-09-28)

| topic | decision |
|---|---|
| scope | **Full in the first PR**: cutout *and* mesh displays (weights included) |
| animation | **Translate keyframes** (key → DB frame, Bézier handles → `curve`). Bake per-frame only where a 1:1 mapping is impossible (IK-driven rotation) |
| styles | **Active style(s) only**, emitted as the default skin (`""`), **one atlas**. Error out if it doesn't fit a single sheet |
| platform | **Native only**; the DragonBones tab is hidden on wasm |
| output | Save dialog picks `<base>_ske.json`; `<base>_tex.json` + `<base>_tex.png` are written next to it. Atlas is always PNG (the JPG option is ignored) |
| IK | Always baked for this exporter (IK family bones get per-frame `rotateFrame`s; the setup pose uses the IK-solved rotation) |
| physics | Dropped |

---

## 2. Target format (authoritative reference)

> Every rule marked **[verified]** below was checked against upstream
> `DragonBonesCPP` (`DragonBones/src/dragonBones/parser/JSONDataParser.cpp`,
> `animation/BaseTimelineState.cpp`, `animation/TimelineState.cpp`,
> `armature/Bone.cpp`, `Cocos2DX_3.x/.../CCSlot.cpp`). Where it conflicted with
> the original draft of this doc, the draft was corrected.

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
- **[verified]** `z` is **ignored** by the parser — draw order is the slot's
  **index in the `slot` array** (first = back). Emit slots sorted by SkelForm
  `zindex` ascending. Animated draw order goes in the animation's `zOrder`
  timeline: `{"frame":[{"duration":n,"zOrder":[slotIdx, offset, ...]}]}` where
  each pair moves slot `slotIdx` to index `slotIdx+offset` (pairs in ascending
  `slotIdx`; listing every slot is valid).
- DragonBones has **bone → slot → display**; SkelForm has no slot concept, so
  **synthesize one slot per display-bearing bone** (§4), named after the bone.

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
  "vertices": [x0,y0, x1,y1, ...],         // slot-local, px
  "uvs": [u0,v0, u1,v1, ...],              // normalized 0..1, relative to the SubTexture REGION
  "triangles": [0,1,2, 2,3,0, ...],        // uint indices
  // weighted skin (FFD/skinning):
  "weights": [ /* per-vertex: boneCount, (boneIdx, weight)... */ ],
  "bonePose": [ /* per bound bone: boneIdx, a,b,c,d,tx,ty */ ],
  "slotPose": [ a,b,c,d,tx,ty ] }
```
**[verified]** mesh packing (`_parseMesh`, `CCSlot::_updateFrame/_updateMesh`):
- `uvs` are relative to the display's SubTexture region (`region.x + u*region.width`),
  **not** the whole atlas. SkelForm's `Vertex.uv` is already region-relative with
  v pointing down, so it maps 1:1.
- `weights`: **every** vertex appears, as `n, (boneIdx, w) × n`. `boneIdx` is the
  index into the armature's `bone` array.
- `bonePose`: stride **7** per bound bone: `boneIdx, a, b, c, d, tx, ty`.
- At load, each vertex is taken to armature space by `slotPose`, then into each
  bound bone's space by `inverse(bonePose[bone])`. At runtime the vertex is
  `Σ w · boneGlobalMatrix · local`, and the slot's own transform is ignored.
- Matrices use `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`.
- Only `translateFrame`/`rotateFrame`/`scaleFrame` + `displayFrame`/`colorFrame`
  and `zOrder` are needed; SkelForm has no per-vertex animation, so no `ffd`.

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
  next). **[verified]** A timeline's first frame always starts at frame 0. The
  last frame's `duration` is ignored: it lasts until the animation's top-level
  `duration`, which is the total frame count.
- **[verified]** If the last frame has a tween and the animation loops, it tweens
  **back to frame 0's value** (`BaseTimelineState::_onArriveAtFrame`). SkelForm
  holds the last value instead, so the exporter emits the last frame with **no**
  `tweenEasing`/`curve`.
- `playTimes`: `0` = loop, `1` = once, n = n times. **[verified]** The default
  when absent is **1**, so the exporter always writes `0` to loop.
- **[verified]** `tweenEasing`: absent = hold (no tween); `0` = linear; `<0` quad
  in; `(0,1]` quad out; `>1` quad in-out.
- **[verified]** `"curve"` takes precedence over `tweenEasing`. It holds cubic
  Bézier **control points**, not samples: `[x1,y1, x2,y2]` for one segment, with
  P0=(0,0) and P3=(1,1) implied. The parser samples it into
  `durationFrames+1` points. SkelForm's `interp()` uses the same easing
  model, so `curve = [start_handle.x, start_handle.y, end_handle.x, end_handle.y]`
  of the **next** keyframe (SkelForm stores a segment's easing on its end key).
- **[verified]** Frame values are **relative to the setup pose**
  (`Bone.cpp`: `global = origin + animationPose`):
  - translate `x`/`y` are px **added** to the setup position,
  - `rotate` is degrees **added** to the setup rotation,
  - scale `x`/`y` **multiply** the setup scale.
- **[verified]** `rotateFrame` interpolates along the **shortest path**
  (`normalizeRadian(delta)`) unless the *previous* frame has `"clockwise": n`
  (n ≠ 0). If a segment turns more than 180°, the exporter writes the raw
  (un-normalized) cumulative angle and puts `clockwise: ±1` on the segment's
  start frame. With that, the parser's correction gives back exactly the raw
  angle.
- The animation-level `frameRate` is **not** read. All animations play at the
  armature's `frameRate` (§4).

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

**Dispatch to wire into (as built):**
1. `Saving` enum (`shared.rs`): add `DragonBones`.
2. `utils::open_save_dialog`: add `("json", "DragonBones")`.
3. Save loop (`lib.rs`, the `saving != Saving::None` branch): `Saving::DragonBones`
   goes to `App::save_dragonbones` (native only), which clones the armature and
   writes the three files on a worker thread, like `App::save`.
4. Export modal (`src/export_modal.rs`): add a 4th **DragonBones** tab. The tabs
   reuse `SettingsState` variants as ids, and this one uses the free
   `SettingsState::Rendering`. The tab is not shown on wasm. Its Export button
   calls `open_save_dialog(.., Saving::DragonBones)`.
5. No separate File-menu item: *File > Export* already opens the export modal.
   The i18n keys live under `export_modal.dragonbones` in `assets/i18n/en.json`.

**PSD import (context; already done)**: `src/file_reader.rs` `read_psd` (~L206)
turns PSD groups into bones, flattening each group into one texture. `$pivot` and
`$ik_*` marker layers configure pivot and IK. This is why the pipeline works
today; the exporter is the reverse trip.

**Shape:** `src/dragonbones_export.rs`:
`pub fn export(armature: &Armature, base: &str) -> Result<DbFiles, String>`
builds `{ ske_json, tex_json, tex_png }`. It is pure (no I/O, no GPU), so it
is unit-testable.

---

## 4. Model mapping + fidelity gotchas

**Coordinate system [verified in SkelForm source]:** SkelForm is **Y-up**
(`read_psd` negates PSD Y; `create_tex_rect` puts the texture's top row at +y).
DragonBones is **Y-down**. Conversion: `y_db = -y`, `rot_db = -rot` (in degrees).
Scale is unchanged, and UVs are unchanged (both have v pointing down, region-relative).

**Bone inheritance [verified at runtime]:** SkelForm composes transforms per component:
`renderer::inheritance` does `rot += parent.rot`, `scale *= parent.scale`, and
`pos = parent.pos + rotate(pos * parent.scale, parent.rot)`. DragonBones multiplies
full matrices, which **skews** a rotated child under a non-uniform parent scale
(e.g. a squash-and-stretch root). SkelForm never skews. The exporter matches
SkelForm this way:
- It sets `"inheritScale": false` on every child bone.
- With that flag, DragonBones still places the child through the parent's matrix
  and adds the rotations, but it takes the child's scale as-is.
- So each bone's `scX/scY` is the **product of the SkelForm scales along its
  ancestor chain**.
- Scale timelines follow the same rule. If exactly one bone in the chain has scale
  keys (the usual root squash), the keys still translate 1:1, just scaled by a
  constant. Otherwise the scale product is sampled every frame.

| SkelForm | DragonBones | notes |
|---|---|---|
| `Bone` (flat, `parent_id`) | `bone` (by `parent` **name**) | resolve id→name; root omits `parent`. Bone names are de-duplicated (`name`, `name_2`, …) because DragonBones looks bones up by name |
| `Bone.rot` **radians** | `transform.skX = skY`, `rotateFrame.rotate` **degrees** | `deg = -rad * 180/PI` (Y flip) |
| `Bone.pos` / `Bone.scale` | `transform.x/y`, `scX/scY` | `y` negated |
| `Bone.tex` (name, resolved via active styles like `Armature::tex_of`) | slot + skin display | one slot per bone that ever shows a texture (setup or `Texture` keys). Display list = the distinct textures it uses |
| `pivot_pos/rot/scale` | display `transform` (image) or baked into vertices (mesh) | image: `x = w*pivot.x`, `y = -(h*pivot.y)`, `skX = skY = -deg(pivot_rot)`, `scX/scY = pivot_scale`. Exception: see "Pivot helpers" below |
| `Bone.zindex` | slot array order + `zOrder` timeline | slots sorted by (zindex, bone order). `Zindex` keys become `zOrder` frames |
| `hidden` (propagates to children) | slot `displayIndex = -1` | combine the bone's own `Hidden` keys with its ancestors' |
| `Texture` keys (`value_str`) | `displayFrame.value` = index in the slot's display list | an empty or unknown texture becomes `-1` |
| `tint` r/g/b/a (0..1) | slot `color` / `colorFrame` `rM/gM/bM/aM` (**int** %) | `round(v*100)` |
| mesh without binds | `type:"mesh"`, no weights | vertices = the pivot transform applied to `Vertex.pos`, Y-flipped |
| mesh with binds | weighted `type:"mesh"` | see "Weights" below |
| path binds (`is_path`) | helper bone per bind, sampled | see "Path binds" below |
| `Style` | skin `""` | active style(s) only, merged by name the same way `tex_of` resolves them |
| `Texture` (`offset`/`size` after `create_tex_sheet`) | `SubTexture` | `SubTexture.name` = texture name |
| `Animation.fps` | armature `frameRate` | DragonBones has one rate per armature. We use the max fps across animations and rescale the other animations' frame numbers (rounded) |
| flat `(bone_id, element)` keyframes | `translateFrame` / `rotateFrame` / `scaleFrame` / `displayFrame` / `colorFrame` / `zOrder` | see "Keyframe translation" below |
| IK families | baked `rotateFrame`s | see below |
| `Physics` | — | dropped |

**Weights (SkelForm semantics → DragonBones).** In `renderer::construct_verts`,
a vertex starts on its owner bone. Each bind then **lerps** it toward
`inherit_vert(local, bindBone)` by `w`, in order. In both cases the same local
coordinate is placed in the other bone's frame, not the bind-pose-relative one.
So each bind multiplies the weights collected so far by `(1-w)` and adds `w` to
its own bone; the owner starts at 1. The result is a normal linear blend in which
**every bound bone sees the same local vertex**.

In DragonBones this means every `bonePose` must equal the `slotPose`. We use the
owner bone's setup world matrix for both, so
`inverse(bonePose)·slotPose·v = v` for every bone. The pivot offset (`pivot_pos`)
is rotated only by the owner bone, which DragonBones can't express on a skinned
mesh. It is folded into the local vertex instead, which is exact when every weight
goes to the owner and approximate otherwise.

**Path binds [verified at runtime].** A path bind puts a vertex at
`bindBone.worldPos + rotate(vertex * w, normal)`, where `normal` is the average
normal of the segments from the previous bind bone to the next one
(`renderer::get_path_normal_angle`). The rubber-hose limbs in `_skellington.skf` use
this. DragonBones can't compute that normal, so:
- Each path bind becomes a root-level helper bone named `<bone>__path<i>`. Its
  position (the bind bone's world position) and rotation (the normal) are
  **sampled every frame**.
- Each path-bound vertex is weighted 1.0 to its helper, with the local coordinate
  `vertex * w`, plus the setup pivot offset.
- A weight bind that comes *after* a path bind for the same vertex is ignored for
  that vertex. DragonBones stores only one local position per vertex/bone pair, so
  it can't express that case.

**Pivot helpers [verified at runtime].** SkelForm draws a texture with two quirks:
- The pivot offset is added in **world axes**
  (`rotate(size*pivot, rot) * worldScale`, see `renderer.rs` `final_pivot`).
- `pivot_rot` is applied **after** the world scale
  (`rotate(v * scale * pivot_scale, rot + pivot_rot)`).

DragonBones applies a display's transform *before* the bone's. The two only differ
while the bone's world scale is **non-uniform** (|sx| ≠ |sy|).

So a slot whose bone has a pivot (`pivot_pos` or `pivot_rot`) and ever goes
non-uniform (in the setup or any animation frame) is moved onto a root-level
`<bone>__pivot` helper bone:
- Its position, rotation (`rot + pivot_rot`) and scale (`scale * pivot_scale`) are
  sampled every frame.
- Its display transform becomes identity.
- Weighted meshes don't use pivot helpers.

**Keyframe translation.**
- SkelForm keys hold **absolute** values. DragonBones frames are relative to the
  setup pose: translate `v - rest`, rotate `v - rest`, scale `v / rest`.
- **Before the first key**, SkelForm holds the first key's value. If that key
  isn't at frame 0, a hold frame is inserted at 0.
- **After the last key**, SkelForm holds; the last DragonBones frame is written
  without a tween (see §2).
- A segment's easing comes from the **end** key's `start_handle`/`end_handle`:
  - Snap (`y == 999`) → no tween.
  - Handles on the diagonal (`x == y` for both) → `tweenEasing: 0`. This covers
    the Linear preset and legacy all-zero handles. SkelForm's 5-step Newton solver
    is slightly off on all-zero handles near t=0, by up to ~3 px on Skellina; the
    exported linear tween is the mathematically correct curve.
  - Anything else → `curve: [sx, sy, ex, ey]`, with x clamped to [0,1].
- **Multi-channel frames:** translate (X/Y), scale (X/Y) and color (R/G/B/A)
  each share one easing per frame.
  - If the channels' key frames and handles match, or all but one channel has no
    keys, the translation is exact.
  - Otherwise that track is **sampled every frame** with linear tweens.
- **Duplicate keys on one frame** (seen in `_skellina.skf`): SkelForm tweens toward
  the *first* one, then holds the *last*. That track is sampled every frame.
- The animation's `duration` is the last keyframe's frame, because SkelForm loops
  when it reaches it (`Animation::set_frame`). `playTimes` is 0.
- **IK**: SkelForm solves IK every frame, so bones in an IK family get their
  rotation **sampled every frame**. The rotation comes from
  `Armature::animate` + `renderer::construction`, as local = world − parent world,
  and it replaces their own Rotation keys. The setup pose uses the IK-solved
  rotation too.
- All sampled tracks drop frames that sit between two identical values.

---

## 5. Acceptance / testing

### 5.1 What was run (2026-09-28)

- `cargo test --test dragonbones_export` runs:
  - structural checks: references resolve, and every field DragonBonesCPP reads with
    `GetInt`/`GetUint` is an integer
  - a synthetic rig covering a weighted mesh, a 360° spin, mismatched X/Y keys,
    texture swaps, inherited hide, zOrder and tint
  - `samples/_skellington.skf` and `samples/_skellina.skf`
  - Set `DB_OUT_DIR=<dir>` to also write the three files plus `<name>_ref.json`,
    SkelForm's own world-space pose for every frame (in DragonBones space).
- **Runtime comparison:** a headless harness built on upstream **DragonBonesCPP**
  (the runtime Inkternity embeds, compiled with MSVC with no renderer):
  - It loads the exported files and samples every frame of every animation.
  - It dumps bone matrices and slot vertices (world space), plus display index,
    draw order and color.
  - These were diffed against `<name>_ref.json`. Max error over every
    frame of every animation:

| rig | bone position | vertices | visibility / draw order / color |
|---|---|---|---|
| synthetic | 0.14 px | 0.33 px | exact |
| skellington (61 bones, 7 meshes incl. path binds, IK) | 0.01 px | 0.01 px | exact |
| skellina (32 bones, squash-and-stretch + pivots) | 3.1 px* | 0.57 px | exact |

\* Only where SkelForm's own Bézier solver is off on legacy all-zero handles (§4).
The sub-pixel residue comes from DragonBones sampling `curve`s into
`frames+1` points.

The harness and scripts are in `tools/dragonbones_verify/`. See
`docs/DRAGONBONES_VERIFY.md` for how to build and run them
(`python tools/dragonbones_verify/verify.py`).

### 5.2 Original checklist

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
