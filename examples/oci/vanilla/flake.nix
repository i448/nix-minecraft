{
  description = "Example vanilla Minecraft OCI image";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nix-minecraft.url = "../../.."; # Path to the root of this repo
  };

  outputs = { self, nixpkgs, nix-minecraft }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      lib = nix-minecraft.lib.${system};
      packages = nix-minecraft.packages.${system};
    in {
      packages.${system}.default = lib.buildImage {
        package = packages.vanilla-server;
      };
    };
}