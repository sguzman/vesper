{
  description = "Vesper — Rust/Bevy development, packaging, and opt-in per-crate Nix builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    crate2nix.url = "github:nix-community/crate2nix";
    crate2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, crane, crate2nix, ... }:
    let
      # Vesper currently targets Linux desktop graphics/audio.
      systems = [ "x86_64-linux" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f system);

      forSystem =
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          inherit (pkgs) lib;
          craneLib = crane.mkLib pkgs;

          nativeLibraries = with pkgs; [
            alsa-lib
            udev
            libxkbcommon
            wayland
            fontconfig
            vulkan-loader
            libGL
          ];
          graphicsLibraries = with pkgs; [
            libGL
            vulkan-loader
            wayland
            libxkbcommon
          ];

          # Excludes artwork from Cargo compilation inputs. Games are copied
          # separately into the Nix distribution, never embedded in the engine.
          cargoSource = craneLib.cleanCargoSource ./.;
          commonArgs = {
            src = cargoSource;
            strictDeps = true;
            nativeBuildInputs = with pkgs; [ pkg-config ];
            buildInputs = nativeLibraries;
          };

          # Separate Cargo artifact pools so a CLI-only Nix build never
          # spends time compiling Bevy. These are per-package, not per-crate,
          # derivations; crateGraph below provides true per-crate caching.
          mkDeps =
            package:
            craneLib.buildDepsOnly (
              commonArgs
              // {
                pname = "${package}-deps";
                version = "0.1.0";
                cargoExtraArgs = "-p ${package}";
              }
            );

          mkBinary =
            package: binary: deps:
            craneLib.buildPackage (
              commonArgs
              // {
                pname = package;
                version = "0.1.0";
                cargoArtifacts = deps;
                cargoExtraArgs = "-p ${package} --bin ${binary}";
                doCheck = false; # Existing full Cargo/Bevy CI owns runtime tests.
              }
            );

          player = mkBinary "vesper-player" "vesper-player" (mkDeps "vesper-player");
          cli = mkBinary "vesper-cli" "vesper" (mkDeps "vesper-cli");

          # Per-crate cache graph, separate from Crane. crate2nix uses IFD to
          # regenerate Cargo.nix from the *locked* Cargo workspace as needed.
          # Kept as explicit opt-in outputs while native Bevy compatibility is
          # certified; plain Cargo/mbx remains the local development fast path.
          crateGraph = crate2nix.tools.${system}.appliedCargoNix {
            name = "vesper";
            src = ./.;
          };

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
            # These two use distinct derivations for every transitive Rust crate.
            per-crate-cli = crateGraph.workspaceMembers."vesper-cli".build;
            per-crate-player = crateGraph.workspaceMembers."vesper-player".build;
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
            LD_LIBRARY_PATH = lib.makeLibraryPath graphicsLibraries;
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
