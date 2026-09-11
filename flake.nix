{
  description = "watsup — Watson time tracker in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.watsup = pkgs.rustPlatform.buildRustPackage {
          pname = "watsup";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          doCheck = false;
          meta.mainProgram = "watsup";
        };

        packages.default = self.packages.${system}.watsup;

        apps.watsup = flake-utils.lib.mkApp { drv = self.packages.${system}.watsup; };
        apps.default = self.apps.${system}.watsup;

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.fzf
          ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };

        checks.default = self.packages.${system}.watsup;
      }
    );
}
