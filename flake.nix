{
  description = "Basic Rust dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = [
          pkgs.cargo
          pkgs.rust-analyzer # LSP
          pkgs.clippy # Linter
          pkgs.rustfmt # Formatter
          pkgs.rustc # Rust compiler
          pkgs.cargo-watch # Auto-rebuild
        ];

        shellHook = ''
          echo "$(cargo --version)"
        '';
      };
    });
}
