{
  description = "Example Fabric Minecraft OCI image";

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
        flavor = "fabric";
        package = packages.fabric-server;
        symlinks = {
          "mods/fabric-api.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/99v969vN/fabric-api-0.111.0%2B1.21.1.jar";
            hash = "sha256-R47y6/Xb3/r1m5/S0P3E/uP8A/CjI/UoF7p/Z/X/Y="; # Example hash
          };
        };
      };
    };
}
