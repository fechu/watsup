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
          meta = {
            mainProgram = "watsup";
            description = "Watson time tracker in Rust";
            homepage = "https://github.com/fechu/watsup";
            license = pkgs.lib.licenses.mit;
          };
        };

        packages.default = self.packages.${system}.watsup;

        apps.watsup = {
          type = "app";
          program = "${self.packages.${system}.watsup}/bin/watsup";
          meta.description = "Watson time tracker in Rust";
        };
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
