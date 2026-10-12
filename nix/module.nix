{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.services.melody;
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
      };
    };
in
{
  options.services.melody = {
    enable = mkEnableOption "Melody music player environment configuration and themes";

    package = mkOption {
      type = types.package;
      default = pkgs.melody or (pkgs.callPackage ./package.nix { });
      defaultText = literalExpression "pkgs.melody";
      description = "The melody package to install.";
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          theme_name = "Midnight Amber";
          primary_font = "Inter";
          page_size = 50;
          volume = 0.8;
          is_shuffle = false;
          is_repeat = false;
        }
      '';
      description = ''
        Configuration written to {file}`~/.config/melody/config.toml` for users.
      '';
    };

    themes = mkOption {
      type = types.attrsOf (types.submodule themeSubmodule);
      default = { };
      example = literalExpression ''
        {
          "dracula" = {
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
        }
      '';
      description = ''
        Custom themes to install into system-wide and user theme directories.
      '';
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    # Install custom themes and default configuration files
    environment.etc =
      let
        renderedThemes = mapAttrs' (
          fileName: themeData:
          nameValuePair "melody/themes/${fileName}.toml" {
            source = tomlFormat.generate "${fileName}.toml" themeData;
          }
        ) cfg.themes;

        renderedConfig = optionalAttrs (cfg.settings != { }) {
          "melody/config.toml" = {
            source = tomlFormat.generate "melody-config.toml" cfg.settings;
          };
        };
      in
      renderedThemes // renderedConfig;
  };
}
