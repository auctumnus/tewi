# Flake for a Rust web application with PostgreSQL and frontend. To use:
# - change `description` below
# - change `name`, `version`, `description` in the flake itself
# - update Cargo.toml
# - update templates/
# Notably, the description can't be set with one variable,
# because flakes are an odd subset of Nix.
# https://github.com/NixOS/nix/issues/4945
#
# For your convenience, each variable across all files is named
# in SCREAMING_SNAKE_CASE.
{
  description = "oomfie image board";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        name = "tewi";
        version = "0.1.0";
        description = "oomfie image board";

        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "llvm-tools-preview"
          ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          just
          cargo-llvm-cov
          bun
          sqlx-cli
          postgresql
          docker-compose
          watchexec
          concurrently
        ];

        buildInputs =
          with pkgs;
          [
            openssl
            postgresql
          ]
          ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          inherit version nativeBuildInputs buildInputs;
          pname = name;

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          # Build frontend as part of the build process
          preBuild = ''
            cd frontend
            bun install --frozen-lockfile
            bun run build
            cd ..
          '';

          meta = {
            inherit description;
            license = pkgs.lib.licenses.mit;
            maintainers = [ ];
          };
        };

        packages.${name} = self.packages.${system}.default;
      }
    );
}
