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
          "mods/c2me.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/VSNURh3q/versions/olrVZpJd/c2me-fabric-mc1.21.11-0.3.6.0.0.jar";
            hash = "sha256-DwWNNWBfzM3xl+WpB3QDSubs3yc/NMMV3c1I9QYx3f8=";
          };

          "mods/chunky.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/fALzjamp/versions/1CpEkmcD/Chunky-Fabric-1.4.55.jar";
            hash = "sha256-M8vZvODjNmhRxLWYYQQzNOt8GJIkjx7xFAO77bR2vRU=";
          };

          "mods/clumps.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/Wnxd13zP/versions/OgBE8Rz4/Clumps-fabric-1.21.11-29.0.0.1.jar";
            hash = "sha256-4yESCFKYF+XvzQ4u6W+cjFFui7reWihs1rii9OcPYWM=";
          };

          "mods/distant-horizons.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/uCdwusMi/versions/GT3Bm3GN/DistantHorizons-2.4.5-b-1.21.11-fabric-neoforge.jar";
            hash = "sha256-dpTHoX5V9b7yG0VsIqKxxOSAYLN0Z97itx1MEuWGvD8=";
          };

          "mods/fabric-api.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/gB6TkYEJ/fabric-api-0.140.2%2B1.21.11.jar";
            hash = "sha256-t8RYO3/EihF5gsxZuizBDFO3K+zQHSXkAnCUgSb4QyE=";
          };

          "mods/ferritecore.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/uXXizFIs/versions/eRLwt73x/ferritecore-8.0.3-fabric.jar";
            hash = "sha256-yG6rrNvwY5ibLKgSyOk/VWuP7/HJ38B8rvodkKXHvzU=";
          };

          "mods/krypton.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/fQEb0iXm/versions/O9LmWYR7/krypton-0.2.10.jar";
            hash = "sha256-lCkdVpCgztf+fafzgP29y+A82sitQiegN4Zrp0Ve/4s=";
          };

          "mods/lithium.jar" = pkgs.fetchurl {
            url = "https://cdn.modrinth.com/data/gvQqBUqZ/versions/gl30uZvp/lithium-fabric-0.21.2%2Bmc1.21.11.jar";
            hash = "sha256-MQZjnHPuI/RL++Xl56gVTf460P1ISR5KhXZ1mO17Bzk=";
          };
        };
      };
    };
}
