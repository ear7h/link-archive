{ pkgs ? import <nixpkgs> {} }:
let
  customBuildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override {
    defaultCrateOverrides = pkgs.defaultCrateOverrides // {
      "link-archive" = attrs: {
        buildInputs =
          if pkgs.stdenv.isDarwin
          then [ pkgs.darwin.apple_sdk.frameworks.Security ]
          else [];
      };
    };
  };
  generatedBuild = import ./Cargo.nix {
    inherit pkgs;
    buildRustCrateForPkgs = customBuildRustCrateForPkgs;
  };
in
  generatedBuild.rootCrate.build

