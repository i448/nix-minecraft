{
  lib,
  rustPlatform,
  buildFeatures ? [ ],
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../../mc-manager/Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = ../../mc-manager;

  cargoLock = {
    lockFile = ../../mc-manager/Cargo.lock;
  };

  inherit buildFeatures;

  meta = with lib; {
    description = "A simple Minecraft server manager for Nix-based OCI images";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "mc-manager";
  };
}
