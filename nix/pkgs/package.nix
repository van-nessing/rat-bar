{
  lib,
  rustPlatform,
  pkg-config,
  pipewire,
  path ? ./.,
}:
let
  config = lib.importTOML (path + /Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = config.package.name;
  version = config.package.version;
  cargoLock.lockFile = path + /Cargo.lock;
  doCheck = false;
  src = path;

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
