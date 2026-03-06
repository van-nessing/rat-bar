{
  lib,
  writers,
  kitty,
  rat-bar,
  path ? ./.,
}:
writers.writeNuBin "rat-bar-scripts" {
  makeWrapperArgs = [
    "--prefix"
    "PATH"
    ":"
    "${lib.makeBinPath [
      kitty
      rat-bar
    ]}"
  ];
} (path + /scripts.nu)
