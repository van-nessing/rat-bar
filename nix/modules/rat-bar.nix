{
  lib,
  pkgs,
  config,
  ...
}:
let
  cfg = config.programs.rat-bar;
  yamlFormat = pkgs.formats.yaml { };
  layout = yamlFormat.generate "layout.yaml" cfg.layout;
  providers = yamlFormat.generate "providers.yaml" cfg.providers;

  type = type: attrs: { ${type} = attrs; };
  mod = prev: mod: builtins.mapAttrs (key: val: val // mod) prev;
  width = width: prev: mod prev { inherit width; };
  no-center = prev: mod prev { center = false; };
  text = t: type "Text" t;
  vgroup =
    elements:
    type "VGroup" {
      inherit elements;
    };
  hgroup =
    elements:
    type "HGroup" {
      inherit elements;
    };
  bar =
    var: direction:
    type "Bar" {
      inherit var direction;
    };
  graph =
    var:
    type "Graph" {
      inherit var;
    };
  image =
    var: width:
    type "Image" {
      inherit var width;
    };
  provider = t: type "Provider" t;
in
{
  options.programs.rat-bar = {
    enable = lib.mkEnableOption "rat-bar";

    service.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      example = false;
      description = "Enable systemd service to automatically start bars on all screens.";
    };

    service.height = lib.mkOption {
      type = lib.types.int;
      default = 4;
      description = "Amount of lines to spawn bar with. Can be resized after.";
    };

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.rat-bar;
      description = "Package to use for rat-bar.";
    };

    layout = lib.mkOption {
      description = "Defines the bar layout.";
      default = [
        {
          block.title = "VISUALIZER";
          component_type = type "Visualizer" { };
        }
        {
          block.title = "CLOCK";
          constraint = type "Length" 30;
          component_type = provider {
            provider = "clock";
            layout = [
              # 1
              (hgroup [
                (text "\${day}")
                (text "\${time}")
                (text "\${date}")
              ])
              # 2
              (hgroup [
                (vgroup [
                  (text "DAY")
                  (text "\${day}")
                ])
                (vgroup [
                  (text "TIME")
                  (text "\${time}")
                ])
                (vgroup [
                  (text "DATE")
                  (text "\${date}")
                ])
              ])
            ];
          };
        }
        {
          block.title = "NOW PLAYING";
          constraint = type "Length" 70;
          component_type = provider {
            provider = "now-playing";
            layout = [
              # 1
              (hgroup [
                (image "art" 2)
                (
                  hgroup [
                    (text "$ul(\${title}) | $ul(\${artist})")
                  ]
                  |> width { Percentage = 70; }
                )
                (text "\${buttons}")
                (bar "progress" "Horizontal" |> width { Percentage = 30; })
                (text "\${position}/\${length}")
              ])
              # 2
              (hgroup [
                (image "art" 5)
                (
                  vgroup [
                    (text "\${title}")
                    (text "$ul(\${album}) | $ul(\${artist})")
                  ]
                  |> width { Percentage = 60; }
                )
                (type "VGroup" {
                  width = (type "Percentage" 40);
                  elements = [
                    (hgroup [
                      (text "\${buttons}")
                      (text "\${position}/\${length}")
                    ])
                    (bar "progress" "Horizontal")
                  ];
                })
              ])
            ];
          };
        }
        {
          block.title = "CPU";
          constraint = type "Length" 35;
          component_type = provider {
            provider = "cpu";
            layout = [
              # 1
              (hgroup [
                (text "LOAD: \${load}%")
                (text "FREQ: \${freq}GHZ")
                (bar "load" "Horizontal")
              ])
              # 2
              (hgroup [
                (vgroup [
                  (text "LOAD")
                  (text "\${load}%")
                ])
                (vgroup [
                  (text "FREQ")
                  (text "\${freq}GHZ")
                ])
                (graph "acc")
              ])
            ];
          };
        }
        {
          block.title = "MEM";
          constraint = type "Length" 27;
          component_type = provider {
            provider = "mem";
            layout = [
              # 1
              (hgroup [
                (text "\${used}GB/\${total}GB")
                (bar "percent" "Horizontal")
              ])
              # 2
              (hgroup [
                (vgroup [
                  (text "FREE")
                  (text "\${available}GB")
                ])
                (vgroup [
                  (text "USED")
                  (text "\${used}GB")
                ])
                (vgroup [
                  (text "TOTAL")
                  (text "\${total}GB")
                ])
                (bar "percent" "Vertical")
              ])
            ];
          };
        }
      ];
      type = lib.types.listOf lib.types.anything;
    };
    providers = lib.mkOption {
      description = "Defines the providers used by rat-bar.";
      default =
        let
          providers = pkgs.rat-bar-providers |> lib.getExe;
        in
        {
          cpu.command = [
            providers
            "cpu"
            "1sec"
            "''"
            "12"
          ];
          now-playing.command = [
            providers
            "now-playing"
            "1sec"
            "paused"
            "[chromium,firefox]"
          ];
          mem.command = [
            providers
            "mem"
            "1sec"
          ];
          clock.command = [
            providers
            "clock"
            "1sec"
          ];
        };
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            command = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              description = "Command and its arguments to run provider.";
            };
          };
        }
      );
    };
  };
  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."rat-bar/layout.yaml".source = layout;
    xdg.configFile."rat-bar/providers.yaml".source = providers;

    systemd.user.services.rat-bar = lib.mkIf cfg.service.enable {
      Unit = {
        After = [ "graphical-session.target" ];
        X-Restart-Triggers = [
          config.xdg.configFile."rat-bar/layout.yaml".source
          config.xdg.configFile."rat-bar/providers.yaml".source
        ];
        StartLimitBurst = 3;
        StartLimitIntervalSec = 10;
      };
      Install = {
        WantedBy = [ "default.target" ];
      };
      Service = {
        Type = "simple";
        ExecStart = "${lib.getExe pkgs.rat-bar-scripts} spawn all ${lib.toString cfg.service.height}";
        Restart = "on-failure";
        RestartSec = 1;
      };
    };
  };
}
