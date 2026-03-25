{
  lib,
  writers,
  playerctl,
  root ? ./.,
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
} (root + /example-config/providers.nu)
