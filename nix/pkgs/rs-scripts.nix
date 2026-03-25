{
  lib,
  rustPlatform,
  makeWrapper,
  kitty,
  rat-bar,
  root ? ./.,
}:
let
  path = root + /ratbar-scripts;
  config = lib.importTOML (path + /Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = config.package.name;
  version = config.package.version;
  cargoLock.lockFile = root + /Cargo.lock;
  cargoBuildFlags = "-p ratbar-scripts-rs";
  doCheck = false;
  src = root;

  nativeBuildInputs = [ makeWrapper ];
  buildInputs = [ ];
  runtimeInputs = [
  ];
  postInstall = ''
    wrapProgram $out/bin/ratbar-scripts-rs \
      --prefix PATH : ${
        lib.makeBinPath [
          kitty
          rat-bar
        ]
      }
  '';
  meta = {
    mainProgram = "ratbar-scripts-rs";
    description = "";
    homepage = "https://github.com/van-nessing/rat-bar";
    license = lib.licenses.mit;
    maintainers = [ ];
  };
})
