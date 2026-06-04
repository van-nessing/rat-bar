{ self }:
{
  lib,
  pkgs,
  config,
  ...
}:
let
  inherit (self.lib)
    kdlNodeToString
    bar-element
    block
    layout
    group
    text
    bar
    graph
    image
    ;
  cfg = config.programs.rat-bar;
  yamlFormat = pkgs.formats.yaml { };
  kdlFormat =
    file: nodes: pkgs.writeText file (lib.strings.concatMapStringsSep "\n" (kdlNodeToString 0) nodes);
  layoutCfg = kdlFormat "layout.kdl" (cfg.layout);
  providers = yamlFormat.generate "providers.yaml" cfg.providers;
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

    package = lib.mkPackageOption pkgs "rat-bar" { };
    scripts-package = lib.mkPackageOption pkgs "ratbar-scripts-rs" { };

    service.height = lib.mkOption {
      type = lib.types.int;
      default = 4;
      description = "Amount of lines to spawn bar with. Can be resized after.";
    };

    layout = lib.mkOption {
      description = "Defines the bar layout.";
      default = [
        (bar-element [
          (block { title = "VISUALIZER"; })
          (layout 1 [
            (graph "visualizer.bins" {
              fill = false;
              marker = "Braille";
            })
          ])
        ])
        (bar-element [
          (block { title = "CLOCK"; })
          (layout 1 [
            (group "h" [
              (text "\${clock.day}")
              (text "\${clock.time}")
              (text "\${clock.date}")
            ])
          ])
          (layout 2 [
            (group "h" [
              (group "v" [
                (text "DAY")
                (text "\${clock.day}")
              ])
              (group "v" [
                (text "TIME")
                (text "\${clock.time}")
              ])
              (group "v" [
                (text "DATE")
                (text "\${clock.date}")
              ])
            ])
          ])
        ])
        (bar-element [
          (layout 1 [
            (group "h" [
              (text "\${media.title} | $[ul](\${media.album}) - $[ul](\${media.artist})")
              (group "h" { width = "8"; } [
                (text "⏮ " { on-click = "media.prev"; })
                (text "\${media.button_symbol} " { on-click = "media.play"; })
                (text "⏭ " { on-click = "media.next"; })
              ])
              (text "\${media.position}/\${media.length}")
            ])
          ])
          (layout 2 [
            (group "h" [
              (image "media.art" { width = 5; })
              (group "v" { width = "3#"; } [
                (text "\${media.title}")
                (text "$[ul](\${media.album}) | $[ul](\${media.artist})")
              ])
              (group "v" { width = "2#"; } [
                (group "h" [
                  (group "h" { width = "8"; } [
                    (text "⏮ " { on-click = "media.prev"; })
                    (text "\${media.button_symbol} " { on-click = "media.play"; })
                    (text "⏭ " { on-click = "media.next"; })
                  ])
                  (text "\${media.position}/\${media.length}")
                ])
              ])
            ])
          ])
        ])
        (bar-element [
          (block { title = "CPU"; })
          (layout 1 [
            (group "h" [
              (text "LOAD: \${cpu.load}%")
              (text "FREQ: \${cpu.FREQ}GHZ")
              (bar "h" {
                var = "cpu.load";
                fg = "blue";
                bg = "dark-gray";
              })
            ])
          ])
          (layout 2 [
            (group "h" [
              (group "v" [
                (text "LOAD")
                (text "\${cpu.load}%")
              ])
              (group "v" [
                (text "FREQ")
                (text "\${cpu.freq}GHZ")
              ])
              (graph "cpu.acc" {
                fg = "blue";
                marker = "Octant";
                fill = true;
              })
            ])
          ])
        ])
        (bar-element [
          (block { title = "MEM"; })
          (layout 1 [
            (group "h" [
              (text "\${mem.used}GB/\${mem.total}GB")
              (bar "v" {
                var = "mem.percent";
                fg = "yellow";
                bg = "dark-gray";
              })
            ])
          ])
          (layout 2 [
            (group "h" [
              (group "v" [
                (text "FREE")
                (text "\${mem.free}GB")
              ])
              (group "v" [
                (text "USED")
                (text "\${mem.used}GB")
              ])
              (group "v" [
                (text "TOTAL")
                (text "\${mem.total}GB")
              ])
              (bar "v" {
                var = "mem.percent";
                fg = "yellow";
                bg = "dark-gray";
              })
            ])
          ])
        ])
        (bar-element [
          (block { title = "NET"; })
          (layout 1 [
            (group "h" [
              (text "RX: \${net.recv}")
              (text "TX: \${net.sent}")
            ])
          ])
          (layout 2 [
            (group "v" { center = false; } [
              (text "RX: \${net.recv}MB/S")
              (text "TX: \${net.sent}MB/S")
            ])
          ])
        ])
      ];
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
          media.command = [
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
    home.packages = [ cfg.package ];
    xdg.configFile."rat-bar/layout.kdl".source = layoutCfg;
    xdg.configFile."rat-bar/providers.yaml".source = providers;

    systemd.user.services.rat-bar = lib.mkIf cfg.service.enable {
      Unit = {
        After = [ "graphical-session.target" ];
        X-Restart-Triggers = [
          config.xdg.configFile."rat-bar/layout.kdl".source
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
