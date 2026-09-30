# DragonBones import

**Goal:** let SkelForm open DragonBones rigs (`<name>_ske.json` + `<name>_tex.json` +
`<name>_tex.png`) as regular SkelForm armatures: bones, textures, styles and
animations, editable and saved as `.skf` like any other rig. It is the reverse of the
common "author in DragonBones Pro, play elsewhere" flow. Rigs from the DragonBones
ecosystem can move *into* SkelForm and then run on SkelForm's own runtimes.

Status: **implemented** in `src/dragonbones_import.rs` (File › Open a
`<name>_ske.json`, desktop only). §0 records where the build departed from this
plan and what was measured; the rest of the doc is the original scoping.

> Format facts below were checked against upstream `DragonBonesCPP`
> (`DragonBones/src/dragonBones/parser/JSONDataParser.cpp`, `animation/*TimelineState*`,
> `armature/Bone.cpp`), which is the reference runtime. SkelForm facts are from this
> repo's `src/`.


## 0. As built (2026-09-29)

**Departures from the plan below:**
- **IK is baked, not mapped to SkelForm IK families.** SkelForm's FABRIK/Arc
  solvers don't reproduce DragonBones' analytic solver. So the importer ports the
  runtime's bone update (all inherit flags) and IK solver (`_computeA`/`_computeB`),
  evaluates DragonBones' own pose, and bakes IK chain bones and bones not inheriting
  rotation/translation into per-frame keys. Everything else keeps 1:1 keyframes.
- **The setup pose is shown without IK.** Meshes are bound against the authored
  (un-IK'd) pose (`bonePose`), so keeping it keeps skinning exact. DragonBones
  applies IK even at setup, so only the static setup view differs; every
  animation frame matches.
- **Slots:** a slot's displays share one bone when their placement agrees relative
  to the texture size and none is a mesh. Otherwise each display gets its own
  child bone (`<slot>/<i>`), shown and hidden by texture keys. Empty slots hide via
  an empty texture, because `Hidden` would propagate to child bones.
- **Trimmed atlas regions** are padded back to their frame, so pivots need no
  per-texture correction.
- **Classic meshes:** when every `bonePose` equals the `slotPose` (the bones sit at
  the mesh's own frame, which is how SkelForm's classic per-bind skinning maps to
  DragonBones), the mesh comes back as classic binds. Other weighted meshes become
  Bind Pose meshes (`BIND_POSE.md`).
- **Older formats** (2.x–4.x) get an error pointing at `db2 -t new`.

**Measured** (`tests/dragonbones_import.rs`, every frame against DragonBonesCPP; see
`DRAGONBONES_VERIFY.md`):

| rigs | result |
|---|---|
| 40 of DragonBonesCPP's 43 sample rigs (incl. IK, weighted meshes, display swaps, rotated/trimmed atlases) | ≤ ~1 px, most < 0.1 px |
| `you_xin/body` (`body`) | 20 px on one face mesh: FFD (mesh deform) animation, unsupported and warned |
| `mecha_1004d` | 3.8 px: a rotated display under a non-uniformly scaled bone (skew, R1) |
| `mecha_2903` | 1.06 px: sub-degree skew in the file (editor rounding) |

---

## 1. Scope

**In scope (v1)**
- DragonBones **5.x JSON** (`version`/`compatibleVersion` 5.0 or 5.5). The
  reference parser also accepts 4.0/4.5 (`DataParser::DATA_VERSIONS`), but those use
  a different animation layout. Older files can be upgraded with the official
  converter instead (§7 Q3).
- One armature per import. If the file holds several, the user picks one.
- Bones, slots, skins, image displays, mesh displays (with and without weights),
  IK constraints.
- Animations: translate/rotate/scale, display index, color, zOrder, IK
  (bend direction).
- Texture atlases, including rotated and trimmed SubTextures and several atlas pages.

**Out of scope (v1), reported to the user as warnings**
- The binary `.dbbin` format.
- Nested armature displays (`type: "armature"`), bounding boxes, and paths/meshes
  shared via `share`. `share` could be a cheap follow-up.
- Deform/FFD timelines. SkelForm has no per-vertex animation.
- Events, actions, sound frames, `playTimes`, `fadeInTime`, animation blending/layers.
- Blend modes, and color *offsets* (SkelForm tint is multiply-only).
- IK `weight` < 1 and IK `scale`. SkelForm IK is all-or-nothing.

## 2. Where it plugs in

| step | code |
|---|---|
| File picker filter | `utils::open_import_dialog` (add "DragonBones" `json`) |
| Dispatch by extension | `file_reader::read_import`. A `.json` whose name ends in `_ske.json` routes to the new importer, and the sibling `_tex.json`/`_tex.png` are found by name |
| Build the armature | new `src/dragonbones_import.rs`: `fn import(ske: &str, atlases: Vec<(tex_json, png_bytes)>) -> Result<Armature, String>`. It is pure, with no I/O or GPU access, so it can be unit tested |
| Textures → GPU | reuse `file_reader::create_texture` / `add_texture` (as `read_psd` does) |
| Web | `read_import` gets **one** file on wasm (`getFile()`). See §7 Q4 |

The importer produces the *inflated* runtime `Armature`, the same shape `utils::import`
builds from `.skf`. From there, saving, editing and exporting work unchanged.

## 3. Coordinate conventions

- DragonBones is Y-down with clockwise-positive degrees. SkelForm is Y-up with
  radians. So `y_skf = -y_db` and `rot_skf = -deg2rad(rot_db)`. Scale is unchanged.
- UVs: both are relative to the texture **region**, with v pointing down, so they
  map 1:1.
- DragonBones `skX`/`skY`: rotation = `skY`, skew = `skX - skY`. SkelForm has no skew,
  so skew is dropped with a warning.

## 4. Model mapping

### 4.1 Bones

| DragonBones | SkelForm | notes |
|---|---|---|
| `bone` (by `parent` name) | `Bone` (`parent_id`) | ids assigned in file order. Parents are guaranteed to come before children after DragonBones' own `sortBones`, and we do the same |
| `transform` x/y/skY/scX/scY | `pos` / `rot` / `scale` | Y flip |
| `length` | — | dropped (display only) |
| `inheritTranslation/Rotation/Scale/Reflection: false` | — | not representable. See §6 R2 |

**Inheritance mismatch (R1).** DragonBones multiplies full matrices. SkelForm composes
position, rotation and scale separately (`renderer::inheritance`): scale multiplies
per component and a child is never skewed. The two agree for uniform scales and
single-axis mirroring. Under a **non-uniform** parent scale with a rotated child,
DragonBones produces skew that SkelForm can't show. The import is exact when every
bone's scale is uniform at setup and throughout its animations. Otherwise it is
approximate, with a warning.

### 4.2 Slots and displays

SkelForm has no slot layer: a bone carries at most one texture at a time. The
proposed mapping:

- Each **slot** becomes a **child bone** of its DragonBones parent bone, named after
  the slot.
- The display's `transform` (x/y/rotation/scale) becomes that child bone's local
  transform.
- The image goes on that bone as `tex`.

This keeps the rig's own bones free of display offsets and matches how SkelForm
users rig (bone = part).

- **Several displays per slot:** SkelForm swaps textures with `Texture` keyframes,
  but all displays then share the slot bone's transform. If a slot's displays have
  identical transforms, use one bone plus `Texture` keys. Otherwise create one bone
  per display and toggle them with `Hidden` keys.
- **`displayIndex: -1`** (empty slot) becomes a hidden bone.
- **Pivot:** the DragonBones default `(0.5, 0.5)` is the image centre, which is also
  SkelForm's. A non-default pivot becomes `pivot_pos = (0.5 - px, py - 0.5)` (in
  texture units, Y-up), or is folded into the slot bone position.
- **Draw order:** the slot's index in the `slot` array becomes `zindex`.
- **Slot color:** multipliers `rM/gM/bM/aM` / 100 become `tint`. Offsets are dropped
  with a warning.

### 4.3 Skins → styles

- DragonBones `skin` maps to a SkelForm `Style`.
- Textures are named after the **slot** (or slot + display index), so every style
  offers the same names and switching styles re-skins the rig.
- The default skin (`""` / `"default"`) becomes the first style, which is active.
- **Limit:** a skin that gives a slot a different *number* or *arrangement* of
  displays than the default skin can't be expressed as a pure texture swap. Such
  displays are imported by name, with a warning.

### 4.4 Atlas

- `SubTexture` regions are cropped from `imagePath` (resolved relative to the
  `_tex.json`).
- `rotated: true` regions are rotated back 90° when cropped.
- Trimmed regions (`frameX/frameY/frameWidth/frameHeight`) are padded back to the
  frame size, so the image centre (the pivot) lands where DragonBones puts it.
- Several atlas pages are all loaded. Display names are looked up across all of them.

### 4.5 Meshes and weights (via Bind Pose)

Unweighted meshes map directly:
- `vertices` (slot-local, Y-flipped) → `Vertex.pos`
- `uvs` → `Vertex.uv`
- `triangles` → `indices`
- `verts_edited = true`

**Weighted meshes are imported as bind-posed meshes** (see `BIND_POSE.md`).
- DragonBones uses standard linear blend skinning: each (vertex, bone) pair has its
  own bind-time offset, `inverse(bonePoseⱼ) · slotPose · v`.
- That is exactly what SkelForm's Bind Pose skinning represents. The importer
  therefore goes through the same code path (`bind_pose.rs`), not a scheme of its
  own.

Steps:
1. Build the bones and set the setup pose. It has to match DragonBones' bind pose,
   i.e. each bound bone's `bonePose` (see below).
2. Store each vertex at its rest position in armature space: `slotPose · v`,
   Y-flipped. The mesh bone's pivot is neutral.
3. For each influencing bone, create the flagged `__bind` helper through
   `bind_pose::helper_for` / `sync_helpers`.
4. Convert DragonBones' linear weights to SkelForm's per-bind sequential weights,
   **exactly**. The first bind is 1; bind k gets `wₖ / (w₁+…+wₖ)`.
5. Mark the mesh bind-posed. The `editor.json` flag is set when the project is
   saved.

**Bind pose vs setup pose.** In SkelForm the bind pose always equals the setup pose
(`BIND_POSE.md` §4.3, Blender-style). DragonBones Pro normally binds at the setup
pose, so `bonePose` equals each bone's setup world transform and everything lines
up.
- If a mesh's `bonePose` differs from the setup pose (a mesh bound at another
  pose), the importer can't keep both.
- It keeps the **setup pose** and re-expresses that mesh's rest vertices so they
  deform identically. This is a one-off conversion at import:
  `v_rest' = B_setup · inverse(bonePose) · v_rest`, per dominant bone.
- It's exact for single-bone vertices and approximate for blended ones. The import
  warning summary lists affected meshes.

**Limit:** the non-uniform-scale limit of Bind Pose (`BIND_POSE.md` §5), the same
as R1 for bones.

**Prior art:** other DragonBones importers (Godot's DBI, Unity's
DragonBoneToUnity, the official DragonBones→Spine converter) copy weights across
directly, because their targets support bind-pose skinning natively. With Bind
Pose, SkelForm does too, through its helper bones.

### 4.6 IK

| DragonBones IK | SkelForm |
|---|---|
| `bone` + `target`, `chain: 0` | 1-bone family `[bone]`, `ik_target_id = target` |
| `chain: 1` | 2-bone family `[bone.parent, bone]` |
| `bendPositive` | `ik_constraint` Clockwise / CounterClockwise. Y flip reverses the sense; verify on a sample |
| `weight` < 1, `scale` | not supported, warning (imported as weight 1) |
| IK timeline (`bendPositive` / `weight` frames) | `IkConstraint` keys; weight dropped |

The solver differs too: DragonBones uses an analytic 2-bone solve, while SkelForm
offers FABRIK or Arc. Import as FABRIK. Poses under IK will be close but not
identical (R4).

### 4.7 Animations

- **Timelines:** `translateFrame`, `rotateFrame`, `scaleFrame`, and the older
  5.0-style all-in-one `frame`, which is split into channels.
- **Frame times:** DragonBones gives each frame a **duration**. SkelForm uses
  **absolute** frames: `frame[i] = sum of earlier durations`. The animation's `fps`
  is the armature's `frameRate`.
- **Values:** DragonBones values are **relative to setup**, SkelForm keys are
  **absolute**:
  - `pos = rest + (x, -y)`
  - `rot = rest - deg2rad(rotate)`
  - `scale = rest * (x, y)`
  - Each DragonBones translate/scale frame becomes an X key and a Y key.
- **Rotation direction:** DragonBones takes the **shortest path** between rotate
  frames unless the previous frame has `clockwise`/`tweenRotate`. SkelForm
  interpolates raw radians. So absolute angles are unwrapped the same way the
  parser does (`prev + normalize(delta)`, or `+ 2π·n` with `clockwise`) before
  becoming keys.
- **Easing lives on different ends.** DragonBones puts the easing on the frame that
  *starts* a segment. SkelForm reads it from the key that *ends* it
  (`interpolate_keyframes` uses `next.start_handle/end_handle`). So each frame's
  easing moves to the next key.
- **Easing mapping** (DragonBones → SkelForm handles on the segment's end key):

| DragonBones | SkelForm |
|---|---|
| no `tweenEasing`, no `curve` | Snap (hold) |
| `tweenEasing: 0` | Linear preset |
| `tweenEasing` e < 0 (quad in, `p + |e|(p²−p)`) | **exact** cubic: `(1/3, (1−|e|)/3)`, `(2/3, (2−|e|)/3)` |
| `tweenEasing` 0 < e ≤ 1 (quad out) | **exact** cubic: `(1/3, (1+e)/3)`, `(2/3, (2+e)/3)` |
| `tweenEasing` > 1 (in-out, cosine) | approximated with a cubic |
| `curve: [x1,y1,x2,y2]` | handles directly (same easing model: cubic, P0=(0,0), P3=(1,1)) |
| `curve` with several segments | split: an extra key at each segment joint (value evaluated there), each part keeping its own handles |

- **Looping tail.** When looping, DragonBones tweens from the last frame back to
  frame 0's value. SkelForm holds, and loops at its last key. So a final key at
  `duration` with frame 0's values and the last frame's easing is added to every
  animated channel.
- **Slot timelines:**
  - `displayFrame` becomes `Texture` keys, or `Hidden` keys for per-display bones
    (§4.2). `-1` becomes Hidden.
  - `colorFrame` multipliers become `Tint*` keys.
- **zOrder timeline:** the offsets are resolved to full slot orders per frame, then
  written as `Zindex` keys on the slot bones.
- **Keys in DragonBones frame numbers:** when the import rate matches, frames map
  1:1.

### 4.8 Dropped, with a warning summary

After import, one modal lists everything that couldn't be carried over, grouped
with counts. For example: "12 `__bind` helper bones added for mesh skinning",
"color offsets on 2 slots ignored", "1 armature display skipped".

## 5. Verification

A headless harness (`tools/dragonbones_verify`, see `DRAGONBONES_VERIFY.md`):

1. **Reference = DragonBonesCPP:** run `dbharness` on the *original* DragonBones
   files. It dumps bone matrices and slot vertices for every frame.
2. **Candidate = SkelForm:** import, then compute SkelForm's own constructed pose
   (`construction()` + vertex positions, converted to Y-down).
3. Compare by bone/slot name, per frame, and report the worst error per category.

Test rigs: DragonBonesCPP's samples (`mecha_1004d`: cutout; `you_xin/body`: 7
weighted meshes, 70 bones). They aren't committed as fixtures; the test takes a
folder via `DB_SAMPLES`.

Expected results: cutout rigs within ~0.5 px; blended-weight meshes should match
within ~0.5 px as well; IK is measured separately (R4). Actual results are in §0.

## 6. Risks

| # | risk | impact | mitigation |
|---|---|---|---|
| R1 | Non-uniform scale → skew | a rotated child under squash distorts slightly | warn; exact otherwise |
| R2 | `inherit*: false` bones | pose wrong for those bones | bake those bones' transforms per frame as keys (world → local), with a warning |
| R3 | Blended mesh weights | would deform differently without a bind pose | imported as bind-posed meshes (§4.5, `BIND_POSE.md`); exact except the non-uniform-scale case shared with R1, and meshes bound away from the setup pose |
| R4 | IK solver differences | IK poses near, not identical | import as FABRIK; optional "bake IK to rotation keys" |
| R5 | Format variants (4.x, 5.0 `frame`) | parse failures on older files | 5.x first; clear error for unsupported versions |

## 7. Open questions

1. **Depends on Bind Pose.** Weighted-mesh import needs `BIND_POSE.md` to land
   first. Cutout rigs and unweighted meshes don't.
2. **Multi-display slots.** Is "one bone per display + Hidden keys" acceptable for
   displays with different transforms, or should the importer only use Texture
   swaps and warn?
3. **Older formats.** *Proposed: not supported.* The official
   `dragonbones-tools` converter upgrades 2.3–4.x files to 5.5 (`db2 -t new`). The
   importer's error message for an old version points users there.
4. **Web.** wasm import currently passes one file. Options: accept a `.zip` of the
   three files, a multi-file picker, or native-only at first.
5. **Where it lives in the UI.** File › Import (same picker, new filter), or a
   separate "Import DragonBones…" entry?
6. **Fixtures.** Are DragonBonesCPP sample assets OK to commit under their license,
   or should the tests use only rigs we generate?

## 8. Rough plan

1. Parser and data structs (`serde` for the JSON), atlas cropping (rotated, trimmed)
   → cutout rigs import with the correct setup pose.
2. Animations: translate/rotate/scale, easing mapping, looping tail, display/color,
   zOrder.
3. Skins → styles, multi-display slots.
4. Meshes: unweighted, then weighted as bind-posed meshes (§4.5, after Bind Pose).
5. IK.
6. Warnings modal, docs, tests and the verification pass.

## 9. References

- **DragonBones 5.5 JSON format spec (official):**
  `github.com/DragonBones/Tools`, `doc/dragonbones_json_format_5.5.md`.
- **DragonBonesCPP** (MIT), the reference runtime/parser used for the facts in
  this doc and for verification.
- **DragonBones/Tools** (MIT), `dragonbones-tools` on npm:
  - `src/action/toSpine.ts`: a complete DragonBones → other-format mapping (timelines,
    easing, skins, IK). The main reference for §4.7, and portable with attribution.
  - `src/action/toNew.ts`: the upgrade path for old formats (§7 Q3).
- **Faktor-de-Voure/DBI** (MIT), a DragonBones importer for Godot 4. It made the
  same calls on IK weight (ignored), multi-segment curves (split into Béziers) and
  slot switching.
- **DragonBones/DragonBoneToUnity**, **zhouzhanglin/Bones2D**: DragonBones/Spine →
  Unity animation converters.
