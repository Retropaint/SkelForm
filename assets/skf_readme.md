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

- String fields (`name`, `*_str`) are debugging aid, and not required to be
  parsed. For `*_str` fields, it is recommended to use their integer counterpart

- IK (inverse kinematics) data is stored in the family's respective root bone(s)

- All IK root bone IDs are stored and can be iterated via `ik_root_ids`

- Bone init fields (`init_*`) should be immutable, and are used to reset their
  respective fields if animation blending is implemented (more info in dev docs)
