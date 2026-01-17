{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.05";
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
          sha256 = "sha256-fI47QJM6DqbAypvhr7GczemzmD5Lv/01BkJMhk06L8Q=";
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
              bacon
              mprocs
              pkg-config
              espflash
              probe-rs-tools
            ]);
        };
      }
    );
}
