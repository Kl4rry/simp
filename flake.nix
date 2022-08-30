{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.follows = "naersk/nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        packageName = "simp";

        pkgs = import nixpkgs {
          inherit system;
        };
        deps = with pkgs; [
          git
          pkg-config
          gtk3
          xorg.libxcb
          speechd
          libxkbcommon
          openssl
          rustc
          cargo
        ];
        naersk' = naersk.lib."${system}";
      in
      rec {
        packages.${packageName} = naersk'.buildPackage {
          pname = "${packageName}";
          root = ./.;
          nativeBuildInputs = deps ++ [ pkgs.wrapGAppsHook ];
        };
        defaultPackage = packages.${packageName};

        apps.${packageName} = packages.${packageName};
        defaultApp = apps.${packageName};

        devShell = pkgs.mkShell {
          buildInputs = deps ++ (with pkgs; [
            rustfmt
            clippy
            rust-analyzer
          ]);
        };
      }
    );
}
