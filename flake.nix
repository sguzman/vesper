{
  description = "Vesper — crate2nix per-crate Rust/Bevy builds and game packaging";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crate2nix.url = "github:nix-community/crate2nix";
    crate2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, crate2nix, ... }:
    let
      # Vesper currently targets Linux desktop graphics/audio.
      systems = [ "x86_64-linux" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f system);

      forSystem =
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          inherit (pkgs) lib;
          nativeLibraries = with pkgs; [
            alsa-lib
            udev
            libxkbcommon
            wayland
            fontconfig
            vulkan-loader
            libGL
            # Winit loads these with dlopen at runtime on X11/XWayland.
            libx11
            libxcursor
            libxi
            libxrandr
            libxcb
          ];

          # crate2nix: each Cargo dependency is its own Nix derivation.
          # The same generated workspace graph builds both binaries,
          # with no separate aggregate dependency pool.
          crateGraph = pkgs.callPackage
            (crate2nix.tools.${system}.generatedCargoNix {
              name = "vesper";
              src = ./.;
            })
            {
              # Default nixpkgs overrides cover alsa-sys and libudev-sys.
              # Bevy 0.20's system-font crate has a separate pkg-config
              # dependency which nixpkgs does not currently provide.
              defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                yeslogic-fontconfig-sys = attrs: {
                  nativeBuildInputs = [ pkgs.pkg-config ];
                  buildInputs = [ pkgs.fontconfig pkgs.freetype ];
                };
                wayland-sys = attrs: {
                  nativeBuildInputs = [ pkgs.pkg-config ];
                  buildInputs = [ pkgs.wayland ];
                };
              };
            };

          cli = crateGraph.workspaceMembers."vesper-cli".build;
          playerCrate = crateGraph.workspaceMembers."vesper-player".build;
          actionCrate = crateGraph.workspaceMembers."vesper-action-demo".build;

          # A standalone player must be able to load Winit's X11/Wayland
          # libraries without being launched through a game-bundle wrapper.
          player = pkgs.runCommand "vesper-player" {
            nativeBuildInputs = [ pkgs.makeWrapper ];
          } ''
            mkdir -p "$out/bin"
            makeWrapper "${playerCrate}/bin/vesper-player" "$out/bin/vesper-player" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}"
          '';

          # New genre: an entirely separate executable with external source art.
          actionDemo = pkgs.runCommand "vesper-action-demo" {
            nativeBuildInputs = [ pkgs.makeWrapper ];
          } ''
            mkdir -p "$out/bin" "$out/share/vesper"
            cp -r ${./examples/action-adventure} "$out/share/vesper/action-adventure"
            makeWrapper "${actionCrate}/bin/vesper-action-demo" "$out/bin/vesper-action-demo" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/action-adventure"
            makeWrapper "$out/bin/vesper-action-demo" "$out/bin/vesper-action-demo-host-nvidia" \
              --set VK_DRIVER_FILES /usr/share/vulkan/icd.d/nvidia_icd.json \
              --prefix LD_LIBRARY_PATH : /usr/lib
          '';

          bundle = pkgs.runCommand "vesper-with-games" {
            nativeBuildInputs = [ pkgs.makeWrapper ];
          } ''
            mkdir -p "$out/bin" "$out/share/vesper"
            cp -r ${./game} "$out/share/vesper/game"
            cp -r ${./examples} "$out/share/vesper/examples"
            makeWrapper "${playerCrate}/bin/vesper-player" "$out/bin/vesper" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/game"
            makeWrapper "${playerCrate}/bin/vesper-player" "$out/bin/vesper-barrio" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/examples/una-tarde-en-el-barrio"
            makeWrapper "${playerCrate}/bin/vesper-player" "$out/bin/vesper-lantern" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/examples/little-lantern"
            # Explicit non-NixOS/Arch NVIDIA host-driver bridge. The owner
            # verified these two variables on EndeavourOS with RTX 3090:
            # host Vulkan ICD + host userspace libraries. Do not force this
            # on normal launchers or other GPU vendors.
            # The host's /etc/alsa selects PipeWire, but the Nix ALSA
            # library cannot load host /usr/lib/alsa-lib modules. Provide
            # compatible Nix PipeWire PCM/control plugins for these opt-in
            # non-NixOS launchers, without overriding an explicit user path.
            test -f "${pkgs.pipewire}/lib/alsa-lib/libasound_module_pcm_pipewire.so"
            test -f "${pkgs.pipewire}/lib/alsa-lib/libasound_module_ctl_pipewire.so"
            makeWrapper "$out/bin/vesper-barrio" "$out/bin/vesper-barrio-host-nvidia" \
              --set VK_DRIVER_FILES /usr/share/vulkan/icd.d/nvidia_icd.json \
              --set-default ALSA_PLUGIN_DIR "${pkgs.pipewire}/lib/alsa-lib" \
              --prefix LD_LIBRARY_PATH : /usr/lib
            makeWrapper "$out/bin/vesper-lantern" "$out/bin/vesper-lantern-host-nvidia" \
              --set VK_DRIVER_FILES /usr/share/vulkan/icd.d/nvidia_icd.json \
              --set-default ALSA_PLUGIN_DIR "${pkgs.pipewire}/lib/alsa-lib" \
              --prefix LD_LIBRARY_PATH : /usr/lib
            # CI/headless only. Never force Lavapipe for normal GPU launchers:
            # desktop users must retain their actual Vulkan adapter.
            makeWrapper "${playerCrate}/bin/vesper-player" "$out/bin/vesper-barrio-software" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath (nativeLibraries ++ [ pkgs.mesa ])}" \
              --set VK_DRIVER_FILES "${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64.json" \
              --add-flags "$out/share/vesper/examples/una-tarde-en-el-barrio"
          '';
        in
        {
          packages = {
            default = bundle;
            inherit bundle;
            vesper-player = player;
            vesper-cli = cli;
            action-demo = actionDemo;
          };

          devShells.default = pkgs.mkShell {
            name = "vesper-dev";
            packages = with pkgs; [
              rustc
              cargo
              clippy
              rustfmt
              rust-analyzer
              pkg-config
              nix-output-monitor
              nixfmt
            ] ++ nativeLibraries;
            LD_LIBRARY_PATH = lib.makeLibraryPath nativeLibraries;
          };

          checks = {
            inherit cli;
          };
        };

      perSystem = eachSystem forSystem;
    in
    {
      packages = eachSystem (system: perSystem.${system}.packages);
      devShells = eachSystem (system: perSystem.${system}.devShells);
      checks = eachSystem (system: perSystem.${system}.checks);
      formatter = eachSystem (system: (import nixpkgs { inherit system; }).nixfmt);
      apps = eachSystem (
        system:
        let
          pkg = perSystem.${system}.packages;
        in
        {
          default = {
            type = "app";
            program = "${pkg.bundle}/bin/vesper-barrio";
          };
          barrio = {
            type = "app";
            program = "${pkg.bundle}/bin/vesper-barrio";
          };
          lantern = {
            type = "app";
            program = "${pkg.bundle}/bin/vesper-lantern";
          };
          barrio-host-nvidia = {
            type = "app";
            program = "${pkg.bundle}/bin/vesper-barrio-host-nvidia";
          };
          lantern-host-nvidia = {
            type = "app";
            program = "${pkg.bundle}/bin/vesper-lantern-host-nvidia";
          };
          action-demo = {
            type = "app";
            program = "${pkg.action-demo}/bin/vesper-action-demo";
          };
          action-demo-host-nvidia = {
            type = "app";
            program = "${pkg.action-demo}/bin/vesper-action-demo-host-nvidia";
          };
          barrio-software = {
            type = "app";
            program = "${pkg.bundle}/bin/vesper-barrio-software";
          };
        }
      );
    };
}
