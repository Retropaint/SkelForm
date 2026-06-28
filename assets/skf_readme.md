# SkelForm File Specification for Runtimes

For the full documentation, please check out
[Dev Docs](https://skelform.org/dev-docs). It can be accessed offline if
SkelForm is installed.

Please ensure you are reading the correct docs version (check `version` in
`armature.json`).

The only files necessary for parsing are `armature.json` and `atlasX.png` (where
X is the atlas number).

## Tips

- All IDs are sequential and start at 0. They may be used directly as array
  indexes

- Bone init fields (`init_*`) should be immutable, and are used to reset their
  respective fields if animation smoothing is enabled

- Animation keyframes always store a single unit of any vector field. Example:
  position is stored as `PositionX` and `PositionY` keyframes, separately

- Tint is stored as a Vector4 (red, green, blue, alpha)
