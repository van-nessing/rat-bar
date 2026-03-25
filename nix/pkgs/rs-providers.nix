{
  lib,
  rustPlatform,
  pkg-config,
  pipewire,
  root ? ./.,
}:
let
  path = root + /ratbar-providers;
  config = lib.importTOML (path + /Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = config.package.name;
  version = config.package.version;
  cargoLock.lockFile = root + /Cargo.lock;
  cargoBuildFlags = "-p ratbar-providers-rs";
  doCheck = false;
  src = root;

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [
    pipewire
  ];
  runtimeInputs = [ ];
  meta = {
    mainProgram = "ratbar-providers-rs";
    description = "";
    homepage = "https://github.com/van-nessing/rat-bar";
    license = lib.licenses.mit;
    maintainers = [ ];
  };
})
