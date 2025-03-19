# Workflow

The main files to work with are `ui.rs` and `renderer.rs`.

* `ui.rs` - user interface logic
* `renderer.rs` - core non-UI rendering logic. Designed to be abstracted away from the rest of WGPU

`main.rs` and `lib.rs` are the backbone - `lib.rs` in particular handles all the WGPU rendering. Editing these files is not recommended.
