{
  lib,
  rustPlatform,
  pkg-config,
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
    pipewire
  ];
  runtimeInputs = [ ];
  meta = {
    mainProgram = "rat-bar";
    description = "";
    homepage = "https://github.com/van-nessing/rat-bar";
    license = lib.licenses.mit;
    maintainers = [ ];
  };
})
