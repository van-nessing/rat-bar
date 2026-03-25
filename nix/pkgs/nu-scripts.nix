{
  lib,
  writers,
  kitty,
  rat-bar,
  root ? ./.,
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
} (root + /scripts.nu)
