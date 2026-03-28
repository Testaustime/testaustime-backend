{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      fenix,
      crane,
      treefmt-nix,
      ...
    }@inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);

      perSystem =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          # Use toolchain specified in rust-toolchain.toml
          toolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-zC8E38iDVJ1oPIzCqTk/Ujo9+9kx9dXq7wAwPMpkpg0=";
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

          commonArgs = {
            src = craneLib.cleanCargoSource ./.;
          };

          cargoArtifacts = craneLib.buildDepsOnly (
            commonArgs
            // {
              doCheck = false;
            }
          );

          treefmtEval = treefmt-nix.lib.evalModule pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.nixfmt.package = pkgs.nixfmt-rfc-style;
            programs.taplo.enable = true;
            programs.rustfmt.enable = true;
            programs.rustfmt.package = toolchain;
          };
        in
        {
          inherit
            pkgs
            toolchain
            craneLib
            commonArgs
            cargoArtifacts
            treefmtEval
            ;
        };
    in
    {
      packages = forEachSystem (
        system:
        let
          inherit (perSystem system)
            pkgs
            craneLib
            commonArgs
            cargoArtifacts
            ;
        in
        rec {
          testaustime-backend = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;

              nativeBuildInputs = [
                pkgs.postgresql
                pkgs.diesel-cli
              ];

              checkInputs = [
                pkgs.postgresql
                pkgs.diesel-cli
              ];

              preCheck = ''
                # Set up a temporary PostgreSQL database
                export PGDATA=$(mktemp -d)
                export PGHOST=$PGDATA

                initdb -U postgres
                pg_ctl start -o "-k $PGDATA -h \"\""

                # Create your test database
                createdb -U postgres test_db

                # Set environment variables your tests expect
                export TEST_DATABASE="postgresql://postgres@localhost/test_db?host=$PGDATA"

                diesel database setup --database-url "$TEST_DATABASE" --migration-dir ${./migrations}
              '';
            }
          );

          default = testaustime-backend;

          docker = pkgs.dockerTools.buildLayeredImage {
            name = "ghcr.io/testaustime/testaustime-backend";
            tag = "nix";
            config.Cmd =
              let
                entrypoint = pkgs.writeShellScriptBin "entrypoint.sh" ''
                  while [ 1 ];
                  do
                      ${pkgs.diesel-cli}/bin/diesel database setup --migration-dir ${./migrations} && break;
                  done
                  ${testaustime-backend}/bin/testaustime
                '';
              in
              [ "./${entrypoint}/bin/entrypoint.sh" ];
          };
        }
      );

      checks = forEachSystem (
        system:
        let
          inherit (perSystem system)
            craneLib
            commonArgs
            cargoArtifacts
            treefmtEval
            ;
        in
        {
          testaustime-backend-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          formatting = treefmtEval.config.build.check self;
        }
      );

      formatter = forEachSystem (system: (perSystem system).treefmtEval.config.build.wrapper);

      devShells = forEachSystem (
        system:
        let
          inherit (perSystem system) pkgs craneLib treefmtEval;
        in
        {
          default = craneLib.devShell {
            checks = self.checks.${system};

            packages = [
              pkgs.diesel-cli
              treefmtEval.config.build.wrapper
            ];
          };
        }
      );
    };
}
