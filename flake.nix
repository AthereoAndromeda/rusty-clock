{
  description = "Development flake for Rusty-Clock";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        name = "rusty-clock";
        pkgs = import nixpkgs {inherit system;};
        fenix-pkgs = fenix.packages.${system};
        toolchain = fenix-pkgs.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-qvHLWbWZbdUYMckRS246ShduGXhR3pEZS0cxF+FCuTU=";
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            systemd
          ];

          packages =
            [
              toolchain
              fenix-pkgs.rust-analyzer
            ]
            ++ (with pkgs; [
              just
              bacon
              pkg-config
              espflash
              probe-rs-tools
              cargo-expand
              cargo-binutils
              cargo-bloat
            ]);
        };
      }
    );
}
