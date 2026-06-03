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
        schemaFilter = path: type:
          type == "regular"
            && ((pkgs.lib.hasSuffix ".schema" path)
              || (pkgs.lib.hasSuffix ".asschema" path));
        scriptFilter = path: type:
          (type == "regular" || type == "directory")
            && (builtins.match ".*/scripts(/.*)?" path != null);
        bootstrapFilter = path: type:
          type == "regular" && builtins.match ".*/bootstrap-policy\\.nota$" path != null;
        sourceFilter = path: type:
          (craneLib.filterCargoSources path type)
            || (schemaFilter path type)
            || (scriptFilter path type)
            || (bootstrapFilter path type);
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = sourceFilter;
          name = "source";
        };
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
          test-generated-schema = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "--test generated_schema";
          });
          test-spirit-migration-uses-contract-projection = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "migrations::persona_spirit::version_0_1_0_to_0_1_1::tests::migrates_historical_certainty_records_to_current_magnitude_records -- --exact";
          });
          generated-schema-source-checked-in = pkgs.runCommand "upgrade-generated-schema-source-checked-in" { } ''
            test -f ${src}/schema/lib.schema
            test -f ${src}/schema/lib.asschema
            test -f ${src}/src/schema/lib.rs
            ! grep -R "include!(concat!(env!(\"OUT_DIR\")" ${src}/src ${src}/build.rs
            touch $out
          '';
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
