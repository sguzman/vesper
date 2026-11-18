# Vesper's crate2nix development and packaging

The Nix build path uses **crate2nix exclusively**: each third-party and workspace Rust crate is built as an individual Nix derivation. There is no parallel aggregate Rust build pipeline or dependency pool. Existing local Cargo/`mbx` incremental development remains available, but it is not Vesper's Nix packaging implementation.

## Platform and tooling

The initial Nix target is `x86_64-linux` (including EndeavourOS and Hyprland). The shell provides stable Rust, Cargo, rustfmt, Clippy, rust-analyzer, pkg-config, `nom`, `nixfmt`, and native Bevy development libraries.

```fish
nix develop -c fish
```

Use the existing `mbx` build coordinator for local non-Nix Cargo work. Nix builds use `nom` to show a readable build log. They do not change the user's Cargo target, fish configuration, or host NixOS/Home Manager setup.

## Packages and runnable examples

```fish
nom build .#vesper-cli      # authoring CLI; crate2nix per-crate dependency graph
./result/bin/vesper check-game examples/una-tarde-en-el-barrio

nom build .#vesper-player   # same native Bevy player, using crate2nix
nom build .#bundle          # player plus external games, images, audio, fonts
nix run .#barrio            # English original urban example, normal host GPU
nix run .#lantern           # short English example, normal host GPU
nix run .#barrio-host-nvidia    # explicitly use Arch host NVIDIA Vulkan stack
nix run .#lantern-host-nvidia   # short English example with host NVIDIA
nix run .#barrio-software   # opt-in Mesa Lavapipe for headless/virtual displays
```

`packages.default` is the full bundle. The bundle installs `bin/vesper` (default game), `bin/vesper-barrio` (original example), and `bin/vesper-lantern`. External story scripts and assets are copied to `share/vesper` rather than compiled into Rust. For another author-supplied game directory, build `vesper-player` and run `./result/bin/vesper-player path/to/game` in a suitable graphical environment. The standalone player output is wrapped with the required X11/Wayland runtime library search path independently of the bundled game launchers. The packaged launchers set the runtime graphics library search path, including X11/XWayland and Wayland dependencies. The `barrio-software` launcher explicitly chooses the Nix-packaged Mesa Lavapipe Vulkan ICD; it exists only for CPU/headless display tests. The ordinary `barrio` and `vesper-barrio` launchers do **not** set a Vulkan driver override, so an NVIDIA/AMD/Intel host retains its actual GPU and drivers.

## Host NVIDIA Vulkan on non-NixOS Linux

On EndeavourOS with the NVIDIA RTX 3090, the normal Nix bundle compiled but its Vulkan loader reported `Unable to find a GPU!`. The owner's successful runtime test explicitly selected the host Vulkan ICD and made `/usr/lib` available. Two **opt-in** convenience launchers now apply those same settings without rebuilding the player:

```fish
nix run .#barrio-host-nvidia
nix run .#lantern-host-nvidia
```

They set `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json` and add `/usr/lib` to `LD_LIBRARY_PATH` before invoking the existing Nix-wrapped player. The NVIDIA ICD and driver libraries must **already exist on the host**; nothing is installed or changed outside the Nix store. The game assets, executable and crate2nix cache are unchanged by choosing a launcher. Use these only for hosts with the Arch-style NVIDIA Vulkan location; they are not an automatic NVIDIA version detector and do not involve nixGL. The standard `barrio`, `lantern` and standalone player remain vendor-neutral, and `barrio-software` remains an explicit CI/headless test.

This host-specific runtime route was verified manually with the equivalent environment variables. The new convenience wrappers still require a direct owner hardware test before being called hardware-verified. Native audio output and the ALSA-to-PipeWire plugin warnings are independently tracked; successfully decoding a sound asset does not prove it played through speakers.

## Per-crate caching

Both Rust binaries use the same `crate2nix.tools.${system}.generatedCargoNix` workspace graph, imported through `pkgs.callPackage` with focused native-library overrides and selected using `workspaceMembers`. Each Rust dependency is a distinct Nix store derivation; downstream rebuilds reuse unchanged cached crate outputs. A changed dependency version, compiler, feature set, or native library may require its dependent graph to rebuild.

crate2nix regenerates its `Cargo.nix` from the checked-in `Cargo.lock` by import-from-derivation (IFD). The generated graph is not hand-edited or checked into the repository. Initial evaluation may build a generator derivation. It is therefore more expensive than evaluating an ordinary static flake, and `nix flake check --no-build` is not a quick syntax-only check for all outputs.

Nix store artifacts and the mutable local Cargo/`mbx` `target` directory are independent caches. No Rust package manager, alternative code generator, or host-wide cache migration is introduced.

## Locking and verification

`Cargo.lock` pins Rust dependencies. The checked-in `flake.lock` pins nixpkgs and crate2nix and its required transitive inputs.

```fish
nix-instantiate --parse flake.nix
nix eval --raw .#packages.x86_64-linux.vesper-cli.drvPath
nom build .#vesper-cli
./result/bin/vesper check-game examples/una-tarde-en-el-barrio
nom build .#bundle
```

The independent Nix CI workflow verifies the committed lockfile, builds the crate2nix CLI, validates the original external game, builds the native Bevy bundle, checks that game assets remain external, and boots the packaged example in an Xvfb/Mesa session. The regular Cargo/Bevy CI remains unchanged.

A green Linux virtual-display run does **not** replace owner acceptance on physical Hyprland/RTX 3090, or verify native audio balance, performance, fullscreen, visual polish, or the original story's final approval. Those gates remain in the narrative-adventure roadmap.
