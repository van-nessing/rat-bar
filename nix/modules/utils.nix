{ lib, ... }:
let
  inherit (lib) isBool isString;
  inherit (lib.trivial) boolToString;
in
rec {
  test = [
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
  kdlNodeToString =
    i: node:
    let
      n = if lib.isFunction node then node [ ] else node;
      kdlString =
        v:
        if isString v then
          "\"${v}\""
        else if isBool v then
          boolToString v
        else if isNull v then
          "null"
        else
          toString v;
      mkIndent = indent: lib.strings.replicate indent "  ";
      args' = map kdlString n.args;
      props' = lib.mapAttrsToList (k: v: "${k}=${kdlString v}") n.props;
      children' = lib.concatMapStringsSep "\n" (
        c: "${mkIndent (i + 1)}${kdlNodeToString (i + 1) c}"
      ) n.children;
      children = if n.children == [ ] then "" else " {\n${children'}\n${mkIndent i}}";
      attrs = lib.strings.concatStringsSep " " ([ n.name ] ++ args' ++ props');
    in
    "${attrs}${children}";
  mkNodeInner =
    state: arg:
    if lib.isList arg then
      (state // { children = arg; })
    else if lib.isAttrs arg then
      mkNodeInner (state // { props = arg; })
    else
      mkNodeInner (state // { args = state.args ++ [ arg ]; });
  mkNode =
    name:
    mkNodeInner {
      inherit name;
      args = [ ];
      props = { };
      children = [ ];
    };
  group = mkNode "group";
  text = mkNode "text";
  bar = mkNode "bar";
  image = mkNode "image";
  graph = mkNode "graph";
  block = mkNode "block";
  layout = mkNode "layout";
  bar-element = mkNode "bar-element";
}
