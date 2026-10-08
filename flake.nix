# The openbim-ifc command as a Nix flake (#377).
#
#   nix run github:openbimrs/ifc -- validate model.ifc
#   nix build github:openbimrs/ifc#openbim-ifc
#
# Builds the openbim-ifc-cli crate from source with the toolchain channel of
# rust-toolchain.toml and the dependencies of Cargo.lock. The source is the
# workspace manifests and crates only: no build script reads anything
# outside it (the schema tables are generated into ifc-schema, never read
# from references/). `.github/workflows/nix.yml` runs `nix flake check` and
# builds the package on Linux and macOS.
{
  description = "openbim-ifc: validate, convert and inspect IFC files (STEP and ifcXML)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      inherit (nixpkgs) lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems =
        f:
        lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
      # The channel the gate and CI pin (1.88.0), so the flake compiles
      # with the same compiler.
      channel = (lib.importTOML ./rust-toolchain.toml).toolchain.channel;
      cli = (lib.importTOML ./crates/openbim-ifc-cli/Cargo.toml).package;
      # Every workspace member's manifest must be present for Cargo to load
      # the workspace, so the whole of crates/ and xtask/ goes in.
      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./crates
          ./xtask
        ];
      };
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          toolchain = pkgs.rust-bin.stable.${channel}.minimal;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
          openbim-ifc = rustPlatform.buildRustPackage {
            pname = "openbim-ifc";
            inherit (cli) version;
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "--package"
              "openbim-ifc-cli"
            ];
            # The command's tests read the fixture corpus and run in the
            # gate; the flake's checks run the built binary instead.
            doCheck = false;
            meta = {
              inherit (cli) description homepage;
              license = lib.licenses.agpl3Plus;
              mainProgram = "openbim-ifc";
              platforms = systems;
            };
          };
        in
        {
          inherit openbim-ifc;
          default = openbim-ifc;
        }
      );

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.openbim-ifc;
          meta.description = cli.description;
        };
      });

      checks = forAllSystems (
        pkgs:
        let
          openbim-ifc = self.packages.${pkgs.stdenv.hostPlatform.system}.openbim-ifc;
        in
        {
          inherit openbim-ifc;
          # The built command reports the crate's version and validates a
          # fixture.
          run = pkgs.runCommand "openbim-ifc-run" { } ''
            reported="$(${lib.getExe openbim-ifc} --version)"
            if [ "$reported" != "openbim-ifc ${cli.version}" ]; then
              echo "openbim-ifc --version says '$reported'" >&2
              exit 1
            fi
            ${lib.getExe openbim-ifc} validate ${./test/fixtures/synthetic-bindings/binding_geometry.ifc}
            touch "$out"
          '';
        }
      );

      # The toolchain of rust-toolchain.toml (clippy, rustfmt, the wasm
      # target) and Python for the scripts; the gate needs more (Node for
      # the docs and npm checks), which it reports when missing.
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            pkgs.python3
          ];
        };
      });
    };
}
