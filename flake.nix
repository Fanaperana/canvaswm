{
  description = "CanvasWM — Infinite Canvas Wayland Compositor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        buildInputs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          udev
          libinput
          libseat
          mesa
          drm
          gbm
          xorg.libX11
          xwayland
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
        ];
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "canvaswm";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version or "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          inherit buildInputs nativeBuildInputs;

          postInstall = ''
            install -Dm755 extras/canvaswm-msg $out/bin/canvaswm-msg
          '';

          meta = with pkgs.lib; {
            description = "An infinite-canvas Wayland compositor";
            homepage = "https://github.com/hades/canvaswm";
            license = licenses.mit;
            mainProgram = "canvaswm";
          };
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          packages = with pkgs; [ rust-analyzer cargo-watch ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/canvaswm";
        };
      });
}
