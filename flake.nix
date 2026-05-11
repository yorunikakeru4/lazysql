{
  description = "lazysql development and build flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    treefmt-nix,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };

      treefmtEval = treefmt-nix.lib.evalModule pkgs {
        projectRootFile = "flake.nix";
        programs.alejandra.enable = true;
        programs.rustfmt.enable = true;
        programs.taplo.enable = true;
      };

      lazysql = pkgs.rustPlatform.buildRustPackage {
        pname = "lazysql";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = [
          pkgs.pkg-config
        ];

        buildInputs =
          [
            pkgs.openssl
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

        # Integration tests require an external Postgres service.
        doCheck = false;
      };
    in {
      packages = {
        default = lazysql;
        lazysql = lazysql;
      };

      apps = {
        default = flake-utils.lib.mkApp {drv = lazysql;};
        lazysql = flake-utils.lib.mkApp {drv = lazysql;};
      };

      checks = {
        lazysql = lazysql;
        formatting = treefmtEval.config.build.check self;
      };

      formatter = treefmtEval.config.build.wrapper;

      devShells.default = pkgs.mkShell {
        inputsFrom = [lazysql];
        packages = with pkgs; [
          cargo
          rustc
          clippy
          rust-analyzer
          just
          podman-compose
          treefmtEval.config.build.wrapper
        ];
      };
    });
}
