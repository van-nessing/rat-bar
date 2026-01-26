{
  lib,
  rustPlatform,
  pkg-config,
  dbus,
  pipewire,
}:
let
  config = lib.importTOML ./Cargo.toml;
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = config.package.name;
  version = config.package.version;
  cargoLock.lockFile = ./Cargo.lock;
  doCheck = false;
  src = ./.;

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [
    dbus
    pipewire
  ];
  runtimeInputs = [ ];
  meta = {
    description = "";
    homepage = "";
    license = lib.licenses.mit;
    maintainers = [ ];
  };
})
