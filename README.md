# SkelForm

SkelForm is a 2D animator, architecturally inspired by DragonBones and made in response to a lack of other free alternatives.

![example](example.png)

# Building

Install Rust v1.85.0 or above.

Then, run `cargo run` in the terminal.

## Web

Install Trunk v0.21.7 or above.

Then, run the `web_build.py` script to build the `dist` folder with the necessary files for web distribution.

Notable argument(s):
* `--serve` - Immediately run localhost:8000 after build.

# Documentation

Run `cargo doc --no-deps --open`

This will open a local docs.rs page.

As of 05/05/2025, proper documentation is still sparse. Until then, enjoy comment hunting!

# Acknowledgements

This project was built on top of [matthewjberger/wgpu-example](https://github.com/matthewjberger/wgpu-example).
