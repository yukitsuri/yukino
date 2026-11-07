{
  description = "Yukino programming language compiler and yukic driver";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          rustVersion = pkgs.rustc.version;

          commonAttrs = {
            version = "0.1.0";
            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
              llvmPackages_22.llvm
              llvmPackages_22.clang
            ];

            buildInputs = with pkgs; [
              llvmPackages_22.libclang
              libffi
              zlib
              libxml2
              ncurses
            ];

            LLVM_CONFIG = "${pkgs.llvmPackages_22.llvm.dev}/bin/llvm-config";
            LLVM_SYS_221_PREFIX = pkgs.llvmPackages_22.llvm.dev;
            LIBCLANG_PATH = "${pkgs.llvmPackages_22.libclang.lib}/lib";

            cargoBuildFlags = [
              "--bin"
              "yukino"
              "--bin"
              "yukic"
            ];

            doCheck = false;
          };

          releasePkg = pkgs.rustPlatform.buildRustPackage (commonAttrs // {
            pname = "yukino";

            cargoBuildType = "release";
            RUSTFLAGS = "-C strip=symbols";
            YUKINO_RELEASEBUILD = "1";

            postInstall = ''
              for binary in "$out"/bin/*; do
                if [ -f "$binary" ]; then
                  ${pkgs.llvmPackages_22.llvm}/bin/llvm-strip --strip-all "$binary"
                fi
              done
            '';

            meta = with pkgs.lib; {
              description = "Yukino programming language compiler and yukic driver";
              homepage = "https://github.com/yukitsuri/yukino";
              license = licenses.asl20;
              mainProgram = "yukino";
            };
          });

          debugPkg = pkgs.rustPlatform.buildRustPackage (commonAttrs // {
            pname = "yukino";

            cargoBuildType = "debug";
            YUKINO_RELEASEBUILD = "0";

            postInstall = ''
              mv "$out/bin/yukino" "$out/bin/yukino-dbg"
              mv "$out/bin/yukic" "$out/bin/yukic-dbg"
            '';

            meta = with pkgs.lib; {
              description = "Yukino debug toolchain and compiler driver";
              homepage = "https://github.com/yukitsuri/yukino";
              license = licenses.asl20;
              mainProgram = "yukino-dbg";
            };
          });
        in
        assert pkgs.lib.versionAtLeast rustVersion "1.98.1";
        {
          yukino = releasePkg;
          yukino-debug = debugPkg;
          default = releasePkg;

          release = releasePkg;
          debug = debugPkg;
        }
      );

      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          rustVersion = pkgs.rustc.version;
        in
        assert pkgs.lib.versionAtLeast rustVersion "1.98.1";
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              llvmPackages_22.llvm
              llvmPackages_22.clang
            ];
            buildInputs = with pkgs; [
              llvmPackages_22.libclang
              rustc
              cargo
              clippy
              rustfmt
              libffi
              zlib
              libxml2
              ncurses
            ];

            LLVM_CONFIG = "${pkgs.llvmPackages_22.llvm.dev}/bin/llvm-config";
            LLVM_SYS_221_PREFIX = pkgs.llvmPackages_22.llvm.dev;
            LIBCLANG_PATH = "${pkgs.llvmPackages_22.libclang.lib}/lib";
            YUKINO_RELEASEBUILD = "1";
          };
        }
      );
    };
}