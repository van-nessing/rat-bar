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
} (path + /example-config/providers.nu)
