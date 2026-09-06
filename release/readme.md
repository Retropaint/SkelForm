# Release

This folder contains all\* assets and scripts for release distributions.

\*Web builds are done with `web_build.py`, which is in root.

## Building

Run `release.py`.

This will create a release distribution with your OS as the target.

Compiling for release tends to take really long. If regularly testing the
release script, use `--debug` flag to speed up compilation.

### Windows

Outputs:

- `skelform_windows.zip` (portable)
- `SkelForm Install.exe` - installer compiled from `install.iss` (Inno Setup 7)

### Mac

By default, the mac release only builds `SkelForm.app`. The release script
provides a `-dmg` flag to attempt to build a DMG as well (requires
[create-dmg](https://github.com/create-dmg/create-dmg))

Outputs:

- `SkelForm.app.zip` (portable)
- `SkelForm.dmg` (`-dmg` flag)

### Linux

Please check Linux dependencies in the root README. Those are required for the
release build as well.

The release script can install said dependencies for Ubuntu (`--ubuntudeps`).
This is used in the Github action for Linux builds.

Outputs:

- `skelform_linux.zip` (portable)
