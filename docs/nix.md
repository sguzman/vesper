# Nix development and package builds

Vesper's existing Cargo / `mbx` workflow remains the local incremental development path. This flake adds a reproducible Linux development shell and two **different** Nix caching strategies; it does not change the Rust crates, game manifests, save paths, or the normal GitHub CI.

## Supported platform

Initial target: `x86_64-linux` (including EndeavourOS/Hyprland). Native libraries for Bevy's ALSA, udev, Wayland/XKB, fontconfig, OpenGL and Vulkan are provided by the development shell. The Nix package is not evidence of actual NVIDIA/Hyprland audio, fullscreen or GPU acceptance; those remain separate QA gates.

## Tooling and local development

From a checked-out repository with flakes enabled:

```fish
nix develop -c fish
cargo build --locked --release -p vesper-player
./target/release/vesper-player examples/una-tarde-en-el-barrio
```

In established local workflows use `mbx` as the build coordinator, rather than running an additional independent Cargo build concurrently. The shell provides Rust stable, Cargo, rust-analyzer, Clippy, rustfmt, pkg-config, Nix formatter, `nom`, crate2nix and Bevy's native dependencies. Existing fish configuration and the user's own Cargo/mbx cache are not moved or deleted.

## Nix package outputs

```fish
nom build .#vesper-cli        # native authoring validator
nom build .#vesper-player     # native game engine (pass an external game dir)
nom build .#bundle           # player + external, separately stored game assets
nom build .#per-crate-cli    # individual dependency-crate derivations
nom build .#per-crate-player # experimental Bevy per-crate graph
```

`packages.default` points to `bundle`. The bundle installs `bin/vesper` (default game), `bin/vesper-barrio` (original example) and `bin/vesper-lantern` (short example); assets remain in `share/vesper`, not embedded into Rust. After building, run `./result/bin/vesper-barrio`, or directly `nix run .#barrio` (also available as the default `nix run`) and `nix run .#lantern`. To play an arbitrary external directory, use `./result/bin/vesper-player path/to/game` when building `vesper-player`.

## What is really cached

| Path | Cache granularity | Mechanism |
| --- | --- | --- |
| `cargo` / `mbx` | Rust compiler build artifacts | Existing mutable Cargo target/cache, outside Nix store |
| `vesper-cli` and `vesper-player` | Separate dependency pools for each binary package | Crane's `buildDepsOnly` then `buildPackage` |
| `per-crate-cli` and `per-crate-player` | **A separate Nix derivation per Rust dependency crate** | crate2nix's `workspaceMembers` graph |

Crane is the conventional build path. Its dependency artifacts are **not** individual Nix derivations per transitive crate. For precise crate-level Nix store caching, use the separately exposed crate2nix outputs. They regenerate `Cargo.nix` from `Cargo.lock` via import-from-derivation (IFD), so no generated multi-thousand-line `Cargo.nix` is committed. IFD requires an initial evaluation-time build, can reduce evaluation parallelism, and may need native build overrides for Bevy/system crates. Treat the per-crate **player** as experimental until its whole dependency graph builds and runs successfully.

Rust code changes invalidate their own derivations and dependent crates, not unrelated per-crate outputs. Changes to compiler version, feature sets or native dependencies can still invalidate caches. Nix store artifacts are not transparently shared with `mbx`'s `target` directory: both mechanisms coexist. No global cache relocation is attempted.

## Pinning and verification

`Cargo.lock` pins Rust dependency versions. `flake.lock` must additionally be committed to pin nixpkgs/Crane/crate2nix inputs. The dedicated Nix CI workflow resolves this lockfile and preserves it as the `vesper-nix-flake-lock` artifact so it can be reviewed and committed. Do not claim fully reproducible flake inputs while `flake.lock` is absent.

```fish
nix flake check --no-build
nom build .#vesper-cli
./result/bin/vesper check-game examples/una-tarde-en-el-barrio
nom build .#per-crate-cli
./result/bin/vesper check-game examples/una-tarde-en-el-barrio
```

Note: after a `nom build` of `per-crate-cli`, the `result` symlink refers to that build, so the validator runs in both cases. The workflow uses separate `result-per-crate` to compare them. `nix flake check` builds the basic CLI check; full native Bevy gameplay and physical Wayland review remain under their existing verification procedures.

Nix can be consumed as a flake input by an external NixOS/Home Manager configuration, but Vesper does not own or modify any host-wide system configuration.
