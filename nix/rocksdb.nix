{ nixpkgs ? import <nixpkgs> {} }:
let
  # all of these values can be modified using the override method of this derivation, e.g. `customRocksDb.override { useSnappy = true; }`
  defaultArgs = {
      # defines which version of rocksdb should be downloaded from github
      version = "7.9.2";
      # allows to disable snappy compression
      useSnappy = false;
      # disables the verify_checksum feature of rocksdb (rocksdb provided by librocksdb-sys calls crc32 each time it reads from database)
      patchVerifyChecksum = true;
      # used to patch source code of rocksdb in order to disable its verify_checksum feature
      # it's one of the options supported by rocksdb, but unfortunately rust-wrapper doesn't support setting this argument to `false`
      patchPath = ./rocksdb.patch;
      # forces rocksdb to use jemalloc (librocksdb-sys also forces it)
      enableJemalloc = true;
    };

  result = rocksDbOptions:
    # WARNING this custom version of rocksdb is only build when useCustomRocksDb == true
    # we use a newer version of rocksdb than the one provided by nixpkgs
    # we disable all compression algorithms, force it to use SSE 4.2 cpu instruction set and disable its `verify_checksum` mechanism
    nixpkgs.rocksdb.overrideAttrs (old: {

      src = builtins.fetchGit {
        url = "https://github.com/facebook/rocksdb.git";
        ref = "refs/tags/v${rocksDbOptions.version}";
      };

      version = "${rocksDbOptions.version}";

      patches = nixpkgs.lib.optional rocksDbOptions.patchVerifyChecksum rocksDbOptions.patchPath;

      cmakeFlags = [
          "-DPORTABLE=0"
          "-DWITH_JNI=0"
          "-DWITH_BENCHMARK_TOOLS=0"
          "-DWITH_TESTS=0"
          "-DWITH_TOOLS=0"
          "-DWITH_BZ2=0"
          "-DWITH_LZ4=0"
          "-DWITH_SNAPPY=${if rocksDbOptions.useSnappy then "1" else "0"}"
          "-DWITH_ZLIB=0"
          "-DWITH_ZSTD=0"
          "-DWITH_GFLAGS=0"
          "-DUSE_RTTI=0"
          "-DFORCE_SSE42=1"
          "-DROCKSDB_BUILD_SHARED=0"
          "-DWITH_JEMALLOC=${if rocksDbOptions.enableJemalloc then "1" else "0"}"
      ];

      propagatedBuildInputs = [];

      buildInputs = nixpkgs.lib.optionals rocksDbOptions.useSnappy [nixpkgs.snappy] ++
                    nixpkgs.lib.optionals rocksDbOptions.enableJemalloc [nixpkgs.jemalloc] ++
                    [nixpkgs.git];
      # it allows to export necessary env variables required by some of the rust packages that we use
      # i.e. ROCKSDB_STATIC=1 and ROCKSDB_LIB_DIR=rocksdb/lib
      # required by the `propagatedBuildInputs` mechanism
      setupHook = nixpkgs.writeText "setup-hook.sh" ''
        exportVars() {
            export ROCKSDB_STATIC=1
            export ROCKSDB_LIB_DIR=$1/lib
        }
        addEnvHooks "$hostOffset" exportVars
      '';
    });
in
nixpkgs.lib.makeOverridable result defaultArgs
