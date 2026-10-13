{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self, nixpkgs, crane }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = crane.mkLib pkgs;
      in {

        packages.${system}.default = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
        };

        devShells.${system}.default = pkgs.mkShell {
          packages = [
            pkgs.bacon
            pkgs.cargo
            pkgs.clippy
            pkgs.rust-analyzer
            pkgs.rustc
            pkgs.rustfmt
          ];

          # becomes the env var RUST_SRC_PATH inside `nix develop`,
          # so rust-analyzer can find the std library sources.
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      };
}
