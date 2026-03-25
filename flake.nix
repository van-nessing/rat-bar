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
      root = ./.;
      inherit (nixpkgs) lib;
      systems = lib.systems.flakeExposed;
      eachSystem =
        f: lib.genAttrs systems (system: f (nixpkgs.legacyPackages.${system}.extend self.overlays.default));
    in
    {
      packages = eachSystem (pkgs: {
        default = pkgs.rat-bar;
        inherit (pkgs)
          rat-bar
          ratbar-scripts-rs
          ratbar-scripts-nu
          ratbar-providers-rs
          ratbar-providers-nu
          ;
      });
      devShells = eachSystem (pkgs: {
        default = pkgs.mkShell {
          name = "rat-bar";
          inputsFrom = [
            pkgs.rat-bar
            pkgs.ratbar-providers-rs
            pkgs.ratbar-scripts-rs
          ];
          buildInputs = [
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
          ];
        };
      });
      homeModules = {
        default = import ./nix/modules/rat-bar.nix { overlay = self.overlays.default; };
      };
      overlays.default = final: prev: {
        rat-bar = final.callPackage ./nix/pkgs/rat-bar.nix { inherit root; };
        ratbar-scripts-rs = final.callPackage ./nix/pkgs/rs-scripts.nix { inherit root; };
        ratbar-scripts-nu = final.callPackage ./nix/pkgs/nu-scripts.nix { inherit root; };
        ratbar-providers-rs = final.callPackage ./nix/pkgs/rs-providers.nix { inherit root; };
        ratbar-providers-nu = final.callPackage ./nix/pkgs/nu-providers.nix { inherit root; };
      };
      overlays.rat-bar = final: prev: {
        rat-bar = final.callPackage ./nix/pkgs/rat-bar.nix { inherit root; };
      };
    };
}
