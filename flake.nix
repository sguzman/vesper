{
  description = "Vesper — Rust/Bevy development, packaging, and opt-in per-crate Nix builds";

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
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xorg.libxcb
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
              };
            };

          cli = crateGraph.workspaceMembers."vesper-cli".build;
          player = crateGraph.workspaceMembers."vesper-player".build;

          bundle = pkgs.runCommand "vesper-with-games" {
            nativeBuildInputs = [ pkgs.makeWrapper ];
          } ''
            mkdir -p "$out/bin" "$out/share/vesper"
            cp -r ${./game} "$out/share/vesper/game"
            cp -r ${./examples} "$out/share/vesper/examples"
            makeWrapper "${player}/bin/vesper-player" "$out/bin/vesper" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/game"
            makeWrapper "${player}/bin/vesper-player" "$out/bin/vesper-barrio" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/examples/una-tarde-en-el-barrio"
            makeWrapper "${player}/bin/vesper-player" "$out/bin/vesper-lantern" \
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath nativeLibraries}" \
              --add-flags "$out/share/vesper/examples/little-lantern"
          '';
        in
        {
          packages = {
            default = bundle;
            inherit bundle;
            vesper-player = player;
            vesper-cli = cli;
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
        }
      );
    };
}
