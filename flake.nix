{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";

    crane-flake = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };


  outputs = { self, nixpkgs, crane-flake, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (localSystem:
      let
        cargoTOML = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        pname = cargoTOML.package.name;
        version = cargoTOML.package.version;

        pkgs = import nixpkgs {
          inherit localSystem;
          overlays = [ (import rust-overlay) ];
        };

        crane = crane-flake.mkLib pkgs;
        base = {
          src = crane.cleanCargoSource ./.;
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            pkg-config
          ];
          nativeBuildInputs = with pkgs; [
            libevdev
          ];
        };

        cargoArtifacts = crane.buildDepsOnly (base // {
          pname = "deps";
        });

        crate = crane.buildPackage (base // {
          inherit cargoArtifacts pname;
        });
      in
      {
        checks = { inherit crate; };
        packages.default = crate;
        formatter = nixpkgs.legacyPackages.${localSystem}.nixpkgs-fmt;

        devShells.default = pkgs.mkShell {
          buildInputs = base.buildInputs;
          nativeBuildInputs = base.nativeBuildInputs;
        };

      });
}
