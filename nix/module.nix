{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg =
    if config.services.melody.enable then
      config.services.melody
    else
      config.programs.melody;

  tomlFormat = pkgs.formats.toml { };

  themeSubmodule =
    { ... }:
    {
      freeformType = tomlFormat.type;
      options = {
        name = mkOption {
          type = types.str;
          description = "Theme display name.";
        };

        background = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#161616";
          description = "Main window background hex color.";
        };

        surface = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#191919";
          description = "Surface panel hex color.";
        };

        surface_hover = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#282828";
          description = "Hovered surface hex color.";
        };

        border = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#282828";
          description = "Border outline hex color.";
        };

        primary = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#ffffff";
          description = "Primary accent hex color.";
        };

        accent = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#888888";
          description = "Secondary accent hex color.";
        };

        text = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#f0f0f0";
          description = "Primary foreground text hex color.";
        };

        text_muted = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#787878";
          description = "Secondary / muted text hex color.";
        };

        success = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#50fa7b";
          description = "Success status hex color.";
        };

        danger = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#ff5555";
          description = "Danger / error status hex color.";
        };

        player_bar_bg = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#191919";
          description = "Optional player bar background hex color.";
        };

        player_bar_text = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#f0f0f0";
          description = "Optional player bar text hex color.";
        };

        slider_active = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#ffffff";
          description = "Active slider track and thumb hex color.";
        };

        slider_rail = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "#363636";
          description = "Inactive slider rail hex color.";
        };

        font_family = mkOption {
          type = types.nullOr types.str;
          default = null;
          example = "Inter";
          description = "Optional custom font family name to associate with this theme.";
        };
      };
    };

  themeType = types.either types.path (types.submodule themeSubmodule);
  layoutType = types.either types.path types.lines;

  moduleOptions = {
    enable = mkEnableOption "Melody native desktop music player";

    package = mkOption {
      type = types.package;
      default = pkgs.melody;
      defaultText = literalExpression "pkgs.melody";
      description = "The melody package to install.";
    };

    autostart = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to start Melody automatically at graphical login via XDG autostart.";
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          theme_name = "Midnight Amber";
          layout_name = "Default";
          primary_font = "Inter";
          page_size = 50;
          tile_width = 140;
          album_tile_width = 140;
          artist_tile_width = 140;
          now_playing_cover_size = 260;
          gapless = true;
          watch_library = true;
          tab_order = [
            "Tracks"
            "Lyrics"
            "Albums"
            "Artists"
            "Playlists"
          ];
          scrobble = {
            listenbrainz_enabled = true;
            listenbrainz_token = "my-token";
            submit_now_playing = true;
            scrobble_percent = 50;
          };
        }
      '';
      description = ''
        System-wide default configuration written to {file}`/etc/xdg/melody/config.toml`.
      '';
    };

    themes = mkOption {
      type = types.attrsOf themeType;
      default = { };
      example = literalExpression ''
        {
          # Attrset definition
          dracula = {
            name = "Dracula";
            background = "#282a36";
            surface = "#44475a";
            surface_hover = "#6272a4";
            border = "#6272a4";
            primary = "#bd93f9";
            accent = "#ff79c6";
            text = "#f8f8f2";
            text_muted = "#6272a4";
            success = "#50fa7b";
            danger = "#ff5555";
            player_bar_bg = "#21222c";
            player_bar_text = "#f8f8f2";
            slider_active = "#bd93f9";
            slider_rail = "#44475a";
          };

          # Or referencing an existing file
          nord = ./themes/nord.toml;
        }
      '';
      description = ''
        System-wide themes to install into {file}`/etc/xdg/melody/themes/`.
        Can be specified either as an attribute set or a path to a TOML file.
      '';
    };

    layouts = mkOption {
      type = types.attrsOf layoutType;
      default = { };
      example = literalExpression ''
        {
          # Referencing an existing KDL file
          minimal = ./layouts/minimal.kdl;

          # Or inline KDL configuration text
          compact = '''
            layout name="Compact" description="Compact top bar and player" {
                column {
                    navbar height=28
                    main-view fill=true
                    player-bar height=64
                }
            }
          ''';
        }
      '';
      description = ''
        System-wide KDL layouts to install into {file}`/etc/xdg/melody/layouts/`.
        Can be specified either as an inline string or a path to a `.kdl` file.
      '';
    };
  };
in
{
  options.services.melody = moduleOptions;
  options.programs.melody = moduleOptions;

  config = mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    # Install system-wide configuration, themes, layouts, and optional autostart
    environment.etc =
      let
        renderedConfig = optionalAttrs (cfg.settings != { }) {
          "xdg/melody/config.toml".source =
            tomlFormat.generate "melody-config.toml" cfg.settings;
          "melody/config.toml".source =
            tomlFormat.generate "melody-config.toml" cfg.settings;
        };

        renderedThemes = foldl' recursiveUpdate { } (
          mapAttrsToList (
            fileName: themeData:
            let
              drv =
                if isPath themeData || isDerivation themeData then
                  themeData
                else
                  tomlFormat.generate "${fileName}.toml" themeData;
            in
            {
              "xdg/melody/themes/${fileName}.toml".source = drv;
              "melody/themes/${fileName}.toml".source = drv;
            }
          ) cfg.themes
        );

        renderedLayouts = foldl' recursiveUpdate { } (
          mapAttrsToList (
            fileName: layoutData:
            let
              fileSpec =
                if isPath layoutData || isDerivation layoutData then
                  { source = layoutData; }
                else
                  { text = layoutData; };
            in
            {
              "xdg/melody/layouts/${fileName}.kdl" = fileSpec;
              "melody/layouts/${fileName}.kdl" = fileSpec;
            }
          ) cfg.layouts
        );

        autostartEntry = optionalAttrs cfg.autostart {
          "xdg/autostart/melody.desktop".source = "${cfg.package}/share/applications/melody.desktop";
        };
      in
      renderedConfig // renderedThemes // renderedLayouts // autostartEntry;
  };
}
