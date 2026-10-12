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
  xorg,
}:

let
  runtimeLibs = [
    alsa-lib
    libGL
    vulkan-loader
    wayland
    libxkbcommon
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
    fontconfig
    freetype
    ffmpeg
  ];
in
rustPlatform.buildRustPackage {
  pname = "melody";
  version = "0.1.0";

  src = lib.cleanSource ./..;

  cargoLock = {
    lockFile = ../Cargo.lock;
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
    # Install built-in themes to share directory
    mkdir -p $out/share/melody/themes
    if [ -d themes ]; then
      cp -r themes/* $out/share/melody/themes/
    fi
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
