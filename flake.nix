{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      devenv,
      systems,
      fenix,
      crane,
      ...
    }@inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      packages = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          toolchain = fenix.packages.${system}.minimal.toolchain;

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
        in
        rec {
          devenv-up = self.devShells.${system}.default.config.procfileScript;
          devenv-test = self.devShells.${system}.default.config.test;

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

      devShells = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [
              {
                languages.rust.enable = true;

                packages = [ self.outputs.packages.${system}.testaustime-backend ];
              }
            ];
          };
        }
      );
    };
}
