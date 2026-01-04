{
  description = "An attempt to better support Minecraft-related content for the Nix ecosystem";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }@inputs:
    let
      ourLib = import ./lib { lib = nixpkgs.lib; };

      mkTests =
        pkgs:
        let
          inherit (pkgs.stdenvNoCC) isLinux;
          inherit (pkgs.lib) optionalAttrs mapAttrs;
          callPackage = pkgs.newScope {
            inherit self;
            inherit (self) outputs;
            lib = pkgs.lib.extend (_: _: { our = ourLib; });
          };
        in
        optionalAttrs isLinux (mapAttrs (n: v: callPackage v { }) (ourLib.rakeLeaves ./tests));

      nixosModules = ourLib.rakeLeaves ./modules;
    in
    {
      lib = ourLib;

      overlay = import ./overlay.nix;
      overlays.default = self.overlay;
      inherit nixosModules;

      hydraJobs = {
        checks = { inherit (self.checks) x86_64-linux; };
        packages = { inherit (self.packages) x86_64-linux; };
      };
    }
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
          };
        };
        docs = pkgs.nixosOptionsDoc {
          inherit
            (pkgs.lib.evalModules {
              modules = [
                { _module.check = false; }
                nixosModules.minecraft-servers
              ];
            })
            options
            ;
        };
      in
      rec {
        legacyPackages = import ./pkgs/all-packages.nix pkgs;

        packages = rec {
          inherit (legacyPackages)
            vanilla-server
            fabric-server
            quilt-server
            paper-server
            velocity-server
            minecraft-server
            nix-modrinth-prefetch
            mc-manager
            ;

          docsAsciiDoc = docs.optionsAsciiDoc;
          docsCommonMark = docs.optionsCommonMark;

          oci-vanilla = lib.buildImage {
            package = vanilla-server;
          };

          oci-fabric = lib.buildImage {
            flavor = "fabric";
            package = fabric-server;
            # Example: Adding Fabric API
            symlinks = {
              "mods/fabric-api.jar" = pkgs.fetchurl {
                url = "https://cdn.modrinth.com/data/P7dR8mSH/versions/99v969vN/fabric-api-0.111.0%2B1.21.1.jar";
                hash = pkgs.lib.fakeHash;
              };
            };
          };

          oci = oci-vanilla;

          oci-debug = lib.buildImage {
            package = vanilla-server;
            debug = true;
          };
        };

        lib = ourLib // {
          buildImage = args: ourLib.buildImage (args // {
            inherit pkgs;
            mc-manager = packages.mc-manager;
          });
        };

        checks = mkTests (pkgs.extend self.outputs.overlays.default) // packages;

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
