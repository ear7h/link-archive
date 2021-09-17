{ pkgs ? import <nixpkgs> {} }:
pkgs.rustPlatform.buildRustPackage {
  pname = "link-archive";
  version = "0.1.0";
  cargoSha256 = "14z1hkyl3lwqrmhfd74c07vc41vdlpxvk117jaswmhnsysicxskb";
  # cargoSha256 = "0000000000000000000000000000000000000000000000000000";
  buildInputs = [
    pkgs.sqlite
    pkgs.darwin.apple_sdk.frameworks.Security
  ];
  src = ./.;
}
