{
  lib,
  writers,
  playerctl,
  path ? ./.,
}:
writers.writeNuBin "rat-bar-providers" {
  makeWrapperArgs = [
    "--prefix"
    "PATH"
    ":"
    "${lib.makeBinPath [
      playerctl
    ]}"
  ];
} (path + /providers.nu)
