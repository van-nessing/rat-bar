{
  lib,
  writers,
  playerctl,
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
    ]}"
  ];
}) "/bin/ratbar-providers-nu" (root + /example-config/providers.nu)
# writers.writeNuBin
# "rat-bar-providers"
# {
#   makeWrapperArgs = [
#     "--prefix"
#     "PATH"
#     ":"
#     "${lib.makeBinPath [
#       playerctl
#     ]}"
#   ];
# }
# (root + /example-config/providers.nu)
