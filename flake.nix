{
  description = "devshell for github:lavafroth/envy-rs";
  inputs = {
    flakelight.url = "github:nix-community/flakelight";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flakelight, ... }@inputs:
    flakelight ./. {
        inherit inputs;
        devShells.packages = pkgs: [ pkgs.stdenv.cc ];
        devShells.env.LD_LIBRARY_PATH = pkgs: "${pkgs.stdenv.cc.cc.lib}/lib";
    };
}
