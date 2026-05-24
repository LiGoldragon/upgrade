{
  description = "upgrade - runtime scaffold for the upgrade triad";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        toolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "rustc"
          "rustfmt"
          "clippy"
          "rust-src"
        ];
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        src = craneLib.cleanCargoSource ./.;
        commonArguments = {
          inherit src;
          strictDeps = true;
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArguments;
        package = craneLib.buildPackage (commonArguments // {
          inherit cargoArtifacts;
          meta.mainProgram = "upgrade";
        });
        conceptSchemaCheck = pkgs.writeShellApplication {
          name = "check-concept-schemas";
          runtimeInputs = [ pkgs.python3 ];
          text = ''
            exec python3 ${./scripts/check_concept_schemas.py} "$@"
          '';
        };
      in
      {
        packages.default = package;
        checks = {
          build = craneLib.cargoBuild (commonArguments // { inherit cargoArtifacts; });
          test = craneLib.cargoTest (commonArguments // { inherit cargoArtifacts; });
          test-invocation = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "--test invocation";
          });
          test-binaries = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "--test binaries";
          });
          test-spirit-migration-uses-contract-projection = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "migrations::persona_spirit::version_0_1_0_to_0_1_1::tests::migrates_historical_certainty_records_to_current_magnitude_records -- --exact";
          });
          doc = craneLib.cargoDoc (commonArguments // {
            inherit cargoArtifacts;
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt { inherit src; };
          clippy = craneLib.cargoClippy (commonArguments // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
        };
        apps = {
          check-concept-schemas = flake-utils.lib.mkApp {
            drv = conceptSchemaCheck;
          };
          spirit-sandbox-upgrade-test = flake-utils.lib.mkApp {
            drv = package;
            exePath = "/bin/upgrade-spirit-sandbox-test";
          };
        };
        devShells.default = pkgs.mkShell {
          name = "upgrade";
          packages = [ pkgs.jujutsu pkgs.pkg-config toolchain ];
        };
      });
}
