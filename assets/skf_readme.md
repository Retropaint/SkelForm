# SkelForm File Specification for Runtimes

For the full documentation, please check out
[Dev Docs](https://skelform.org/dev-docs). It can be accessed offline if
SkelForm is installed.

Please ensure you are reading the correct docs version (check `version` in
`armature.json`).

The only files necessary for parsing are `armature.json` and `textures.png`.

## Tips

- All IDs are sequential and start at 0. They may be used directly as array
  indexes

- IK (inverse kinematics) data is stored in the family's respective root bone(s)

- All IK root bone IDs are stored and can be iterated via `ik_root_ids`

- Bone init fields (`init_*`) should be immutable, and are used to reset their
  respective fields if animation blending is implemented (more info in dev docs)

- Animation keyframes always store a single unit of any vector field.
  Example: position is stored as `PositionX` and `PositionY` keyframes,
  separately

- Tint is stored as a Vector4 (red, green, blue, alpha)
