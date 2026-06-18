{
  lib,
  writers,
  playerctl,
  cava,
  nushell,
  root ? ./.,
}:
writers.makeScriptWriter ({
  interpreter = "${lib.getExe nushell} --stdin";
  makeWrapperArgs = [
    "--prefix"
    "PATH"
    ":"
    "${lib.makeBinPath [
      playerctl
      cava
    ]}"
  ];
}) "/bin/ratbar-providers-nu" (root + /example-config/providers.nu)
