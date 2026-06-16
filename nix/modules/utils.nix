{ lib, ... }:
let
  inherit (lib) isBool isString;
  inherit (lib.trivial) boolToString;
in
rec {
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
