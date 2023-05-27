{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs@{ nixpkgs, rust-overlay, ... }:
    let
      eachSystem = systems: f:
        let
          op = attrs: system:
            let
              ret = f system;
              op = attrs: key:
                let
                  appendSystem = key: system: ret: { ${system} = ret.${key}; };
                in attrs // {
                  ${key} = (attrs.${key} or { })
                    // (appendSystem key system ret);
                };
            in builtins.foldl' op attrs (builtins.attrNames ret);
        in builtins.foldl' op { } systems;
      defaultSystems = [ "x86_64-linux" "aarch64-darwin" ];
    in eachSystem defaultSystems (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        src = pkgs.nix-gitignore.gitignoreSource [ ".git" ] ./.;
        package = (pkgs.lib.importJSON (src + "/package.json"));
        rust-stable = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };
        crane = rec {
          lib = inputs.crane.lib.${system};
          stable = lib.overrideToolchain rust-stable;
        };
        bitmessage-rs = crane.stable.buildPackage {
          src = ./.;
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            rust-analyzer
            pkg-config
            glib
            gdk-pixbuf
            pango
            gtk4
            libadwaita
            openssl
            sqlite
            (if system == "aarch64-darwin" then
              [ darwin.apple_sdk.frameworks.SystemConfiguration ]
            else
              [ ])
          ];
          cargoBuildCommand = "cargo build --release";
        };
      in rec {
        packages = { inherit bitmessage-rs; };
        defaultPackage = packages.bitmessage-rs;
        defaultApp = packages.bitmessage-rs;
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust-bin.stable.latest.default
            rust-analyzer
            pkg-config
            glib
            gdk-pixbuf
            pango
            gtk4
            libadwaita
            openssl
            sqlite
            (if system == "aarch64-darwin" then
              [ darwin.apple_sdk.frameworks.SystemConfiguration ]
            else
              [ ])
          ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        };
      });
}
