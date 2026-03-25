# Release

This folder contains all\* assets and scripts for release distributions.

\*Web builds are done with `web_build.py`, which is in root.

## Building

Run `release.py`.

This will create a release distribution with your OS as the target.

### Mac

By default, the mac release only builds `SkelForm.app`. The release script
provides a `-dmg` flag to attempt to build a DMG instead (requires
[create-dmg](https://github.com/create-dmg/create-dmg))

### Linux

Please check Linux dependencies in the root README. Those are required for the
release build as well.

The release script can install said dependencies for Ubuntu (`--ubuntudeps`).
This is used in the Github action for Linux builds.
