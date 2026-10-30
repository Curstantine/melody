{
  lib,
  rustPlatform,
  pkg-config,
  ffmpeg,
  alsa-lib,
  fontconfig,
  freetype,
  libGL,
  vulkan-loader,
  wayland,
  libxkbcommon,
  libx11,
  libxcursor,
  libxi,
  libxrandr,
}:

let
  runtimeLibs = [
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
in
rustPlatform.buildRustPackage {
  pname = "melody";
  version = "0.2.2";

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "cosmic-text-0.19.0" = "sha256-93zdZ04BRwQTKqsdwYtFOaQkOsTzp2Aq4VQhNwFE8cQ=";
      "cryoglyph-0.1.0" = "sha256-KOKHPv+PZkwdMkGoR2nNixUAatUR23VYPW0hLHFQ08M=";
      "iced-0.15.0-dev" = "sha256-E6+9mw/IzTZtLFfFANw63Tt4M/JwXuA6RndiMBHkB6I=";
      "winit-0.30.13" = "sha256-XoB5tbWXkynHe8s5ZSkQRbQb3Lmq+RPi2UtLqh53Q/M=";
    };
  };

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];

  buildInputs = [
    ffmpeg
    alsa-lib
    fontconfig
    freetype
  ] ++ runtimeLibs;

  postInstall = ''
    # Install built-in themes and layouts
    mkdir -p $out/share/melody/themes $out/share/melody/layouts
    if [ -d themes ]; then
      cp -r themes/* $out/share/melody/themes/
    fi
    if [ -d layouts ]; then
      cp -r layouts/* $out/share/melody/layouts/
    fi

    # Install desktop file and icons
    install -Dm644 assets/melody.desktop $out/share/applications/melody.desktop
    install -Dm644 assets/melody.svg $out/share/icons/hicolor/scalable/apps/melody.svg
    install -Dm644 assets/melody.png $out/share/icons/hicolor/256x256/apps/melody.png
  '';

  postFixup = ''
    patchelf --add-rpath "${lib.makeLibraryPath runtimeLibs}" "$out/bin/melody"
  '';

  meta = with lib; {
    description = "Lightweight, cross-platform native desktop music player written in Rust";
    homepage = "https://github.com/Curstantine/melody";
    license = with licenses; [ mit asl20 ];
    mainProgram = "melody";
    platforms = platforms.linux;
  };
}
