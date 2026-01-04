{ pkgs, lib }:
lib.our.buildImage {
  inherit pkgs;
  mc-manager = pkgs.mc-manager; # Exists in pkgs because of overlay
  package = pkgs.vanilla-server;
  serverProperties = {
    server-port = 25565;
    motd = "Nix OCI Server";
  };
}
