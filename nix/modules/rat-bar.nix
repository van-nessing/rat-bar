{ overlay }:
{
  self,
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
    var: direction: fg: bg:
    type "Bar" {
      inherit
        var
        direction
        fg
        bg
        ;
    };
  graph =
    var: fg:
    type "Graph" {
      inherit var fg;
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

    layout = lib.mkOption {
      description = "Defines the bar layout.";
      default = [
        {
          block.title = "VISUALIZER";
          constraint = type "Fill" 1;
          component_type = provider {
            provider = "visualizer";
            layout = [ (graph "bins" "Gray") ];
          };
        }
        {
          block.title = "CLOCK";
          constraint = type "Length" 30;
          component_type = provider {
            provider = "clock";
            layout = [
              #1
              (hgroup [
                (text "\${day}")
                (text "\${time}")
                (text "\${date}")
              ])
              #2
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
              #1
              (hgroup [
                (image "art" 2)
                (width { Percentage = 70; } (
                  hgroup [
                    (text "$[ul](\${title}) | $[ul](\${artist})")
                  ]
                ) # |> width { Percentage = 70; }
                )
                (text "\${buttons}")
                (width { Percentage = 30; } (bar "progress" "Horizontal" "Red" "DarkGray"))
                (text "\${position}/\${length}")
              ])
              #2
              (hgroup [
                (image "art" 5)
                (width { Percentage = 60; } (vgroup [
                  (text "\${title}")
                  (text "$[ul](\${album}) | $[ul](\${artist})")
                ])
                  # |> width { Percentage = 60; }
                )
                (type "VGroup" {
                  width = (type "Percentage" 40);
                  elements = [
                    (hgroup [
                      (text "\${buttons}")
                      (text "\${position}/\${length}")
                    ])
                    (bar "progress" "Horizontal" "Red" "DarkGray")
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
              #1
              (hgroup [
                (text "LOAD: \${load}%")
                (text "FREQ: \${freq}GHZ")
                (bar "load" "Horizontal" "Blue" "DarkGray")
              ])
              #2
              (hgroup [
                (vgroup [
                  (text "LOAD")
                  (text "\${load}%")
                ])
                (vgroup [
                  (text "FREQ")
                  (text "\${freq}GHZ")
                ])
                (graph "acc" "Blue")
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
              #1
              (hgroup [
                (text "\${used}GB/\${total}GB")
                (bar "percent" "Horizontal" "Yellow" "DarkGray")
              ])
              #2
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
                (bar "percent" "Vertical" "Yellow" "DarkGray")
              ])
            ];
          };
        }
        {
          block.title = "NET";
          constraint = type "Length" 16;
          component_type = provider {
            provider = "net";
            layout = [
              #1
              (hgroup [
                (text "RX: \${recv}")
                (text "TX: \${sent}")
              ])
              #2
              (no-center (vgroup [
                (text "RX: \${recv}MB/S")
                (text "TX: \${sent}MB/S")
              ])
                # |> no-center
              )
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
          providers-rs = lib.getExe pkgs.ratbar-providers-rs;
        in
        {
          cpu.command = [
            providers-rs
            "cpu"
            "1sec"
            ""
          ];
          now-playing.command = [
            providers-rs
            "media"
            "1sec"
            "paused"
            "spotify"
          ];
          mem.command = [
            providers-rs
            "mem"
            "1sec"
          ];
          clock.command = [
            providers-rs
            "clock"
            "1sec"
            "day=%a"
            "time=%R"
            "date=%d.%m.%Y"
          ];
          net.command = [
            providers-rs
            "net"
            "1sec"
          ];
          visualizer.command = [
            providers-rs
            "visualizer"
            "10ms"
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
    nixpkgs.overlays = [
      overlay
    ];

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
        ExecStart = "${lib.getExe pkgs.ratbar-scripts-rs} spawn --lines ${lib.toString cfg.service.height}";
        Restart = "on-failure";
        RestartSec = 1;
      };
    };
  };
}
