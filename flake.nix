{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      systems = lib.systems.flakeExposed;
      eachSystem =
        f: lib.genAttrs systems (system: f (nixpkgs.legacyPackages.${system}.extend self.overlays.default));
    in
    {
      packages = eachSystem (pkgs: {
        default = pkgs.rat-bar;
        rat-bar = pkgs.rat-bar;
        rat-bar-scripts = pkgs.rat-bar-scripts;
      });
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          name = "rat-bar";
          inputsFrom = [ pkgs.rat-bar ];
          buildInputs = [
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
          ];
        };
      });
      overlays.default = final: prev: {
        rat-bar = final.callPackage ./package.nix { };
        rat-bar-scripts = final.writers.writeNuBin "rat-bar-scripts" (builtins.readFile ./scripts.nu);
      };
    };
}
