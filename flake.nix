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
      path = ./.;
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
        rat-bar-providers = pkgs.rat-bar-providers;
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
      homeModules = {
        default = import ./nix/modules/rat-bar.nix;
      };
      overlays.default = final: prev: {
        rat-bar = final.callPackage ./nix/pkgs/package.nix { inherit path; };
        rat-bar-scripts = final.callPackage ./nix/pkgs/scripts.nix { inherit path; };
        rat-bar-providers = final.callPackage ./nix/pkgs/providers.nix { inherit path; };
      };
    };
}
