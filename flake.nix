{
  description = "Melody - A Tauri + Rust + Node.js application";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # ── Rust toolchain ────────────────────────────────────────────────────
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-analyzer" "rust-src" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # ── Node / pnpm ───────────────────────────────────────────────────────
        # Pin to a specific major to match your .node-version / packageManager field.
        nodejs = pkgs.nodejs_22;
        # pnpm_9 tracks pnpm 9.x; change to pnpm_10 etc. if needed.
        pnpm = pkgs.pnpm_9;

        # ── System libraries required by Tauri / WebKitGTK ───────────────────
        # Split into native (build-time) vs. non-native (link-time / runtime)
        # so both devShell and crane derivations get the right attributes.
        nativeBuildDeps = with pkgs; [
          pkg-config
          cargo-tauri
        ];

        buildDeps = with pkgs; [
          # TLS / networking
          openssl

          # GTK / WebKit stack
          glib
          gtk3
          cairo
          pango
          gdk-pixbuf
          atk
          libsoup_3
          webkitgtk_4_1

          # App-indicator / tray
          libappindicator-gtk3
          librsvg

          # D-Bus (notifications, system integration)
          dbus

          # X11 / XCB (needed even under Wayland via XWayland or WebKitGTK's X backend)
          libxcb
          libx11
          libxcursor
          libxrandr
          libxi

          # Media
          ffmpeg
        ];

        # ── PKG_CONFIG_PATH helper ────────────────────────────────────────────
        # Gather .dev outputs for packages that need them.
        pkgConfigPath = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" (with pkgs; [
          openssl.dev
          libsoup_3.dev
          glib.dev
          gtk3.dev
          webkitgtk_4_1.dev
          dbus.dev
        ]);

      in
      {
        # ── Dev shell ─────────────────────────────────────────────────────────
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = nativeBuildDeps;
          buildInputs = buildDeps ++ [
            rustToolchain
            nodejs
            pnpm

            # Extra dev-only tools
            pkgs.cargo-watch
            pkgs.cargo-expand
            pkgs.clippy
            pkgs.rustfmt
          ];

          shellHook = ''
            export PKG_CONFIG_PATH="${pkgConfigPath}:$PKG_CONFIG_PATH"
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildDeps}:$LD_LIBRARY_PATH"
          '';
        };

        # ── Packages ──────────────────────────────────────────────────────────
        packages = {
          default = self.packages.${system}.melody;

          # Rust backend crate only (useful for CI caching)
          backend = craneLib.buildPackage {
            src = craneLib.cleanCargoSource ./.;
            cargoExtraArgs = "-p backend";
            nativeBuildInputs = nativeBuildDeps;
            buildInputs = buildDeps;
            PKG_CONFIG_PATH = pkgConfigPath;
          };

          # Full Tauri application bundle
          # ---------------------------------------------------------------
          # Tauri's build tooling (cargo-tauri) calls both `cargo build` and
          # the frontend build internally, so we drive everything through a
          # single stdenv derivation rather than fighting buildNpmPackage's
          # npm-centric fetch with a pnpm workspace.
          #
          # The pnpm store is fetched in a fixed-output derivation so Nix can
          # cache it without network access during the main build.
          # ---------------------------------------------------------------
          melody =
            let
              # 1. Fetch the pnpm store offline.
              #    Run `nix build .#pnpm-store` once, let it fail, then paste
              #    the correct hash reported by Nix here.
              pnpmStore = pkgs.stdenv.mkDerivation {
                name = "melody-pnpm-store";
                src = ./.;

                nativeBuildInputs = [ nodejs pnpm ];

                # Fixed-output derivation: network is allowed, result is hashed.
                outputHashAlgo = "sha256";
                outputHashMode = "recursive";
                # Replace with the real hash after the first failed build:
                outputHash = pkgs.lib.fakeHash;

                buildPhase = ''
                  export HOME=$TMPDIR
                  pnpm fetch --frozen-lockfile
                '';

                installPhase = ''
                  cp -r node_modules $out
                '';
              };
            in
            pkgs.stdenv.mkDerivation {
              pname = "melody";
              version = "0.1.0";
              src = ./.;

              nativeBuildInputs = nativeBuildDeps ++ [ nodejs pnpm rustToolchain ];
              buildInputs = buildDeps;

              PKG_CONFIG_PATH = pkgConfigPath;
              LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildDeps;

              # Disable network in the main build — all deps come from the store above.
              __noChroot = false;

              buildPhase = ''
                export HOME=$TMPDIR

                # Re-use the pre-fetched pnpm store (no network needed).
                cp -r ${pnpmStore} node_modules
                chmod -R u+w node_modules

                pnpm install --frozen-lockfile --offline
                pnpm build:packages

                cargo tauri build --ci
              '';

              installPhase = ''
                mkdir -p $out/bin

                # Tauri places the binary at src-tauri/target/release/<name>.
                # Adjust the binary name if your Tauri identifier differs.
                install -Dm755 src-tauri/target/release/melody $out/bin/melody

                # Optional: also install the .deb / AppImage if you want them
                # as build outputs.
                # mkdir -p $out/share/melody
                # cp src-tauri/target/release/bundle/appimage/*.AppImage $out/share/melody/ 2>/dev/null || true
              '';
            };
        };

        # ── Checks (run with `nix flake check`) ──────────────────────────────
        checks = {
          fmt = craneLib.cargoFmt {
            src = craneLib.cleanCargoSource ./.;
          };

          clippy =
            let
              cargoArtifacts = craneLib.buildDepsOnly {
                src = craneLib.cleanCargoSource ./.;
                nativeBuildInputs = nativeBuildDeps;
                buildInputs = buildDeps;
                PKG_CONFIG_PATH = pkgConfigPath;
              };
            in
            craneLib.cargoClippy {
              inherit cargoArtifacts;
              src = craneLib.cleanCargoSource ./.;
              nativeBuildInputs = nativeBuildDeps;
              buildInputs = buildDeps;
              PKG_CONFIG_PATH = pkgConfigPath;
              cargoClippyExtraArgs = "-- --deny warnings";
            };
        };
      }
    );
}
