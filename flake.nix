{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        runtimeLibs = with pkgs; [
          alsa-lib
          libGL
          vulkan-loader
          wayland
          libxkbcommon
          libx11
          libxcursor
          libxi
          libxrandr
          fontconfig
          freetype
          ffmpeg
        ];

        melodyPkg = pkgs.callPackage ./nix/package.nix { };
      in
      {
        packages.default = melodyPkg;
        packages.melody = melodyPkg;

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            clang
          ];
          buildInputs =
            with pkgs;
            [
              nixd
              nixfmt
              noto-fonts-cjk-sans
              (pkgs.rust-bin.stable.latest.default.override {
                extensions = [
                  "rust-src"
                  "rust-analyzer"
                ];
              })
            ]
            ++ runtimeLibs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        };

        checks.build = melodyPkg;
      }
    )
    // {
      overlays.default = final: prev: {
        melody = self.packages.${final.stdenv.hostPlatform.system}.default;
      };

      nixosModules.default = import ./nix/module.nix;
      nixosModules.melody = self.nixosModules.default;
    };
}
