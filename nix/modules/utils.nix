{ ... }:
rec {
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
    var: fg: marker: fill:
    type "Graph" {
      inherit
        var
        fg
        marker
        fill
        ;
    };
  image =
    var: width:
    type "Image" {
      inherit var width;
    };
  provider = t: type "Provider" t;
}
