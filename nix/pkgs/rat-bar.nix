{
  lib,
  rustPlatform,
  root ? ./.,
}:
let
  path = root + /rat-bar;
  config = lib.importTOML (path + /Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = config.package.name;
  version = config.package.version;
  cargoLock.lockFile = root + /Cargo.lock;
  cargoBuildFlags = "-p rat-bar";
  doCheck = false;
  src = root;

  nativeBuildInputs = [ ];
  buildInputs = [ ];
  runtimeInputs = [ ];
  meta = {
    mainProgram = "rat-bar";
    description = "";
    homepage = "https://github.com/van-nessing/rat-bar";
    license = lib.licenses.mit;
    maintainers = [ ];
  };
})
