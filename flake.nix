{
  description = "Lenchog's epic analyzer with _special_ Skipgrams";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };
  outputs =
    inputs@{
      flake-parts,
      fenix,
      crane,
      devshell,
      treefmt-nix,
      nixpkgs,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      imports = [
        devshell.flakeModule
        treefmt-nix.flakeModule
      ];
      perSystem =
        {
          system,
          config,
          lib,
          ...
        }:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ fenix.overlays.default ];
          };

          packages = {
            default = packages.chogalyzer;
            chogalyzer = craneLib.buildPackage build-attrs // {
              meta.mainProgram = "chogalyzer";
            };
          };

          toolchain =
            with fenix.packages.${system};
            combine [
              latest.rustc
              latest.cargo
              latest.clippy
              latest.rust-analysis
              latest.rust-src
              latest.rustfmt
            ];

          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

          build-deps = with pkgs; [ ];

          unfilteredRoot = ./.; # The original, unfiltered source

          src = lib.fileset.toSource {
            root = unfilteredRoot;
            fileset = lib.fileset.unions [
              # Default files from crane (Rust and cargo files)
              (craneLib.fileset.commonCargoSources unfilteredRoot)
              # Keep assets
              (lib.fileset.maybeMissing ./assets)
            ];
          };

          build-attrs = {
            inherit src;
            buildInputs = build-deps;
          };

          deps-only = craneLib.buildDepsOnly ({ } // build-attrs);

          checks = {
            clippy = craneLib.cargoClippy (
              {
                cargoArtifacts = deps-only;
                cargoClippyExtraArgs = "--all-features -- --deny warnings";
              }
              // build-attrs
            );

            rust-fmt = craneLib.cargoFmt ({ inherit src; } // build-attrs);

            rust-tests = craneLib.cargoNextest (
              {
                cargoArtifacts = deps-only;
                partitions = 1;
                partitionType = "count";
              }
              // build-attrs
            );
          };

        in
        {
          inherit checks packages;
          treefmt = {
            programs = {
              nixfmt-rfc-style.enable = true;
              statix.enable = true;
            };
            flakeFormatter = true;
            projectRootFile = "flake.nix";
          };

          devshells.default = {
            packages =
              with pkgs;
              [
                config.treefmt.build.wrapper
                nixfmt-rfc-style
                statix
                # rust
                gcc # required for clap
								rust-analyzer
								lldb
                toolchain
								(pkgs.python3.withPackages (python-pkgs: [
									python-pkgs.matplotlib
								]))
              ]
              ++ build-deps;
          };

        };
    };
}
