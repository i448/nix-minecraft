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
        debug = true;
        symlinks = {
          "mods/fabric-api.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/gB6TkYEJ/fabric-api-0.140.2%2B1.21.11.jar";
            hash = "sha256-t8RYO3/EihF5gsxZuizBDFO3K+zQHSXkAnCUgSb4QyE=";
          };
        };
      };
    };
}
