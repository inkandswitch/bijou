{
  description = "bijou: bijective variable-length integer encodings";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-26.05";
    nixos-unstable.url = "nixpkgs/nixos-unstable-small";

    flake-utils.url = "github:numtide/flake-utils";

    command-utils = {
      url = "git+https://tangled.org/expede.wtf/nix-command-utils";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Charon + Aeneas: translate the Rust crate to Lean for the refinement
    # proofs in lean-aeneas/. Charon is built from this input; the Aeneas
    # binary comes from the matching nightly release (`aeneas-release`
    # below). Pinned, not following our nixpkgs: Aeneas needs specific
    # OCaml package versions. Bump together with `aeneas-release` and the
    # Aeneas Lean library rev in lean-aeneas/lakefile.toml.
    aeneas.url = "github:aeneasverif/aeneas/69280e07ed995241a4e23ef15937f3a2dea18fd2";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    wasm-bodge-src = {
      url = "github:alexjg/wasm-bodge/v0.5.0";
      flake = false;
    };
  };

  outputs = {
    self,
    aeneas,
    flake-utils,
    nixos-unstable,
    nixpkgs,
    rust-overlay,
    command-utils,
    wasm-bodge-src,
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [
          (import rust-overlay)
        ];

        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };

        unstable = import nixos-unstable {
          inherit system overlays;
          config.allowUnfree = true;
        };

        # Single sources of truth for versions that must agree across files.
        # Each is read from the manifest that owns it so the flake cannot drift.
        workspaceManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        # Toolchain: rust-toolchain.toml (also read by rustup on non-Nix runners).
        rustVersion = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml)).toolchain.channel;

        # Minimum supported Rust version: `workspace.package.rust-version`.
        # rust-overlay keys are full `x.y.z`; Cargo allows `x.y`.
        msrv = let
          v = workspaceManifest.workspace.package.rust-version;
        in
          if builtins.length (pkgs.lib.splitString "." v) == 2
          then "${v}.0"
          else v;

        msrv-toolchain = pkgs.rust-bin.stable.${msrv}.minimal.override {
          targets = ["wasm32-unknown-unknown"];
        };

        # wasm-bindgen-cli must match the exact `=0.2.x` crate pin.
        wasm-bindgen-version =
          pkgs.lib.removePrefix "=" workspaceManifest.workspace.dependencies.wasm-bindgen.version;

        rust-toolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [
            "cargo"
            "clippy"
            "llvm-tools-preview"
            "rust-src"
            "rust-std"
          ];

          targets = [
            "aarch64-apple-darwin"
            "x86_64-apple-darwin"

            "x86_64-unknown-linux-musl"
            "aarch64-unknown-linux-musl"

            "wasm32-unknown-unknown"
            "thumbv6m-none-eabi"
          ];
        };

        # Nightly rustfmt for unstable formatting options (imports_granularity, etc.)
        # We need a combined nightly toolchain (rustc + rustfmt) because rustfmt
        # links against librustc_driver, which lives in the rustc component.
        # On macOS, symlinks break @rpath resolution, so we wrap the binary
        # with DYLD_LIBRARY_PATH pointing to the combined toolchain's lib/.
        nightly-rustfmt-unwrapped = pkgs.rust-bin.nightly.latest.minimal.override {
          extensions = ["rustfmt"];
        };

        nightly-rustfmt = pkgs.writeShellScriptBin "rustfmt" ''
          export DYLD_LIBRARY_PATH="${nightly-rustfmt-unwrapped}/lib''${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
          export LD_LIBRARY_PATH="${nightly-rustfmt-unwrapped}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          exec "${nightly-rustfmt-unwrapped}/bin/rustfmt" "$@"
        '';

        # Pinned-toolchain Rust platform for building helper crates from source.
        # Reused by wasm-bodge and gungraun-runner; both need our edition-2024
        # rust-overlay toolchain rather than the nixpkgs default.
        pinnedRustPlatform = pkgs.makeRustPlatform {
          cargo = rust-toolchain;
          rustc = rust-toolchain;
        };

        wasm-bodge = pinnedRustPlatform.buildRustPackage {
          pname = "wasm-bodge";
          version = "0.5.0";
          src = wasm-bodge-src;
          cargoHash = "sha256-lNgGxyLcO7yEATlMfjjVXBgmjtXI4l+lFk/yagP7lh0=";
          nativeBuildInputs = [unstable.cargo-auditable];
          doCheck = false; # tests require npm/puppeteer infrastructure
        };

        # gungraun-runner: required binary harness for gungraun (formerly
        # iai-callgrind) instruction-count benchmarks. Not yet in nixpkgs;
        # we build it from crates.io. Version must match the `gungraun`
        # workspace dep in `bijoux/Cargo.toml`.
        gungraun-runner = pinnedRustPlatform.buildRustPackage rec {
          pname = "gungraun-runner";
          version = "0.19.4";

          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-DrIbeUVI+fhrp87rzIxYRvAlPSJ3ksa6cHHNFg4I+zE=";
          };

          cargoHash = "sha256-68SL8pEYw9nV9g3ZmUjWDL9DXOBDfSMy1y3ZuKuHW2I=";

          # Tests require a full benchmark execution loop with valgrind.
          # We're shipping just the binary harness.
          doCheck = false;
        };

        # wasm-bindgen-cli MUST match the workspace's `wasm-bindgen` crate
        # exactly (the test runner refuses mismatched schema versions), so
        # build it at the pinned version instead of taking whatever nixpkgs
        # ships. >= 0.2.127 is required for wasm-bodge's panic=unwind mode.
        # When bumping the Cargo.toml pin, update both hashes below (a stale
        # hash fails loudly at build time).
        wasm-bindgen-cli = unstable.buildWasmBindgenCli rec {
          src = unstable.fetchCrate {
            pname = "wasm-bindgen-cli";
            version = wasm-bindgen-version;
            hash = "sha256-di+qBAdd7pENLiIB9CoZoab+W5xeDoByMREcCGTSzWo=";
          };

          cargoDeps = unstable.rustPlatform.fetchCargoVendor {
            inherit src;
            inherit (src) pname version;
            hash = "sha256-FTv2GZIAQs0ePdIZXIXil7JbZ6kIT05VG6vqC1qNFxQ=";
          };
        };

        # Charon: built from source at the revision the Aeneas input pins.
        # `charon-driver` is a rustc plugin linked against one exact nightly,
        # which the upstream flake wires up; a prebuilt binary would need
        # that toolchain patched in by hand.
        charon = inputs.aeneas.packages.${system}.charon;

        # Aeneas: the statically linked binary from the nightly release cut
        # at the same revision as the `aeneas` input (so its LLBC format
        # matches Charon). Avoids a long OCaml build. Upstream publishes
        # these three platforms only.
        aeneas-release = {
          tag = "nightly-2026.11.26-69280e0";
          assets = {
            x86_64-linux = {
              name = "linux-x86_64";
              hash = "sha256-mrdupw0TnrFERNzUj2pak263nmW1EZu4EWgYScXb45U=";
            };
            aarch64-linux = {
              name = "linux-aarch64";
              hash = "sha256-J9j6qNPNVNYEZ9MEJzzSsy9B+7QjfdrFEcLzxhpZhhA=";
            };
            aarch64-darwin = {
              name = "macos-aarch64";
              hash = "sha256-0WfPzhimDNYVTAKujiW5/0CjG7gV1kPQR/7zf3IIlVA=";
            };
          };
        };

        # The proof outputs (packages.{aeneas,charon}, apps aeneas-translate /
        # ci-aeneas, devShells.proofs) exist only where Aeneas publishes a binary.
        has-aeneas = aeneas-release.assets ? ${system};

        # One Aeneas revision is pinned in three places: the `aeneas` input
        # (Charon), the release tag (the binary) and lean-aeneas/lakefile.toml
        # (its Lean library). Likewise the Lean toolchain in lean/ and
        # lean-aeneas/. Checked when the proof tooling is evaluated, so a
        # partial bump fails with a clear message instead of a confusing
        # translate or proof error.
        lakefile-aeneas-rev =
          (pkgs.lib.findFirst (r: r.name == "aeneas") (throw "lean-aeneas/lakefile.toml: no `aeneas` require")
            (builtins.fromTOML (builtins.readFile ./lean-aeneas/lakefile.toml)).require).rev;
        pins-agree = assert pkgs.lib.assertMsg
        (pkgs.lib.hasSuffix (builtins.substring 0 7 inputs.aeneas.rev) aeneas-release.tag)
        "aeneas-release.tag (${aeneas-release.tag}) is not cut at the aeneas input's revision (${inputs.aeneas.rev})";
        assert pkgs.lib.assertMsg (lakefile-aeneas-rev == inputs.aeneas.rev)
        "lean-aeneas/lakefile.toml pins Aeneas ${lakefile-aeneas-rev}, the flake's aeneas input is ${inputs.aeneas.rev}";
        assert pkgs.lib.assertMsg
        (builtins.readFile ./lean/lean-toolchain == builtins.readFile ./lean-aeneas/lean-toolchain)
        "lean/lean-toolchain and lean-aeneas/lean-toolchain differ: bump both together"; true;

        aeneas-bin = let
          asset = aeneas-release.assets.${system} or (throw "No prebuilt Aeneas for ${system}");
        in
          assert pins-agree;
            pkgs.stdenvNoCC.mkDerivation {
              pname = "aeneas";
              version = aeneas-release.tag;
              src = pkgs.fetchurl {
                url = "https://github.com/AeneasVerif/aeneas/releases/download/${aeneas-release.tag}/aeneas-${asset.name}.tar.gz";
                inherit (asset) hash;
              };
              sourceRoot = ".";
              dontConfigure = true;
              dontBuild = true;
              installPhase = ''
                install -Dm755 aeneas $out/bin/aeneas
              '';
            };

        # Nightly toolchain for wasm-bodge's panic=unwind builds, which rebuild
        # std via -Zbuild-std (needs rust-src). Kept minimal + separate from the
        # stable dev toolchain. "latest" is pinned by the rust-overlay input in
        # flake.lock, so this is reproducible.
        nightly-wasm-toolchain = pkgs.rust-bin.nightly.latest.minimal.override {
          # rust-src only: -Zbuild-std compiles std from source, so a
          # prebuilt wasm32 std component would be dead weight.
          extensions = ["rust-src"];
        };

        # wasm-bodge runs bare `wasm-opt -O4`, relying on the module's
        # target_features section. Rust's wasm output uses bulk-memory (stable
        # default since 1.87) and, for panic=unwind builds, legacy exception
        # handling — neither of which binaryen enables by default. Enable them
        # explicitly (validation only; -O4 won't introduce features beyond
        # what the module already uses, keeping Node 20 compatibility).
        # Mirrors the wasm-opt wrapper in .github/workflows/test-wasm.yml —
        # keep the flag lists in sync.
        wasm-opt-shim = pkgs.writeShellScriptBin "wasm-opt" ''
          exec "${pkgs.binaryen}/bin/wasm-opt" \
            --enable-bulk-memory \
            --enable-exception-handling \
            --enable-multivalue \
            --enable-mutable-globals \
            --enable-nontrapping-float-to-int \
            --enable-reference-types \
            --enable-sign-ext \
            "$@"
        '';

        # wasm-bodge invokes `cargo +nightly ...` (a rustup convention). There
        # is no rustup in this shell, so this cargo shim routes
        # `+nightly`-prefixed calls to the pinned nightly toolchain (RUSTC/
        # RUSTDOC included — nightly cargo would otherwise pick up the stable
        # rustc from PATH) and everything else to the stable toolchain. Only
        # prepended to PATH where needed (the `bodge` command).
        cargo-nightly-shim = pkgs.writeShellScriptBin "cargo" ''
          case "$1" in
            +nightly)
              shift
              export RUSTC="${nightly-wasm-toolchain}/bin/rustc"
              export RUSTDOC="${nightly-wasm-toolchain}/bin/rustdoc"
              exec "${nightly-wasm-toolchain}/bin/cargo" "$@"
              ;;
            +*)
              echo "cargo shim: unsupported toolchain override '$1'" >&2
              echo "(this Nix shell only provides stable and +nightly)" >&2
              exit 1
              ;;
          esac
          exec "${rust-toolchain}/bin/cargo" "$@"
        '';

        # ------------------------------------------------------------------
        # CI checks: each is a standalone `nix run .#ci-<name>` app so hosted
        # CI and local runs execute byte-identical commands.
        # ------------------------------------------------------------------

        # `stdenv.cc` is the linker for build scripts and host test binaries;
        # without it the apps silently depend on whatever `cc` the host has.
        ci-env = [
          nightly-rustfmt
          pkgs.stdenv.cc
          rust-toolchain
        ];

        # Rust's std links `-liconv` on darwin. A mkShell puts libiconv on the
        # cc wrapper's search path; these standalone apps have to do it
        # themselves (clang honours LIBRARY_PATH).
        link-env = pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
          export LIBRARY_PATH="${pkgs.libiconv}/lib''${LIBRARY_PATH:+:$LIBRARY_PATH}"
        '';

        mkCheck = name: text:
          pkgs.writeShellApplication {
            name = "bijou-${name}";
            runtimeInputs = ci-env;
            text = ''
              export RUSTFMT="${nightly-rustfmt}/bin/rustfmt"
              ${link-env}
              set -x
              ${text}
            '';
          };

        ci-checks = {
          ci-fmt = mkCheck "ci-fmt" ''
            cargo fmt --all --check
          '';

          ci-clippy = mkCheck "ci-clippy" ''
            # Default features first: the all-features build below would hide
            # a break in the default configuration consumers actually get.
            cargo check --workspace --all-targets
            cargo clippy --workspace --all-targets --all-features -- -D warnings
            # Host clippy never sees `#[cfg(target_arch = "wasm32")]` code.
            # Library targets only for the workspace: `bijoux`'s dev-deps
            # (bolero → getrandom) don't build for wasm32.
            cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings
            cargo clippy --package bijoux_wasm --all-targets --target wasm32-unknown-unknown -- -D warnings
          '';

          ci-test = mkCheck "ci-test" ''
            cargo test --workspace
            cargo test --workspace --all-features
          '';

          ci-doc = mkCheck "ci-doc" ''
            cargo test --doc --workspace --all-features
            RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
          '';

          # Host checks prove the feature gates are self-consistent; the
          # thumbv6m checks prove no_std (that target has no `std` at all, so
          # any dependency that pulls it in fails to build).
          ci-no-std = mkCheck "ci-no-std" ''
            for features in "" u32 u64 u128 i32,i64,i128 u32,u64,u128,i32,i64,i128; do
              cargo check --package bijoux --no-default-features --features "$features"
              cargo check --package bijoux --no-default-features --features "$features" --target thumbv6m-none-eabi
            done
          '';

          ci-wasm = mkCheck "ci-wasm" ''
            cargo check --workspace --target wasm32-unknown-unknown
            # bijoux_wasm's tests too, compile-checked in seconds; ci-wasm-node
            # executes them. (Not `--workspace --tests`: `bijoux`'s dev-deps
            # pull in getrandom, which doesn't build for wasm32.)
            cargo check --package bijoux_wasm --tests --target wasm32-unknown-unknown
          '';

          # Build with the `rust-version` from Cargo.toml (${msrv}) so the
          # advertised MSRV is real. Library consumers resolve fresh rather
          # than using our committed Cargo.lock, so do the same: re-resolve,
          # preferring MSRV-compatible versions, and restore the committed
          # lockfile afterwards. Only the published library (`bijoux`, all
          # features) is held to the MSRV; dev-dependencies (criterion,
          # gungraun, pprof) are free to require newer toolchains.
          ci-msrv = pkgs.writeShellApplication {
            name = "bijou-ci-msrv";
            runtimeInputs = [msrv-toolchain pkgs.coreutils pkgs.stdenv.cc];
            text = ''
              ${link-env}
              export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
              committed_lock="$(mktemp)"
              cp Cargo.lock "$committed_lock"
              trap 'mv "$committed_lock" Cargo.lock' EXIT
              set -x
              cargo generate-lockfile
              cargo --version
              cargo check --package bijoux
              cargo check --package bijoux --all-features
              cargo check --package bijoux --no-default-features
              cargo check --package bijoux_wasm --target wasm32-unknown-unknown
            '';
          };

          # Property tests. BOLERO_RANDOM_ITERATIONS bounds each harness;
          # hosted CI raises it per matrix row (see test-bolero.yml).
          ci-bolero = mkCheck "ci-bolero" ''
            export BOLERO_RANDOM_ITERATIONS="''${BOLERO_RANDOM_ITERATIONS:-1000000}"
            export RUST_BACKTRACE=1
            cargo test --release --workspace --features bolero --lib tests::property
          '';
        };

        # Executes the wasm-bindgen-test suites under Node. wasm-pack picks up
        # the pinned wasm-bindgen-cli from PATH instead of downloading one.
        ci-wasm-node = pkgs.writeShellApplication {
          name = "bijou-ci-wasm-node";
          runtimeInputs = [rust-toolchain wasm-bindgen-cli pkgs.nodejs pkgs.stdenv.cc pkgs.wasm-pack];
          text = ''
            ${link-env}
            set -x
            wasm-pack test --node bijoux_wasm
          '';
        };

        # Lean 4 model of the SPEC (lean/). elan installs the toolchain named
        # in lean/lean-toolchain (lean-aeneas/ pins the same one; `pins-agree`
        # checks), the way rustup reads rust-toolchain.toml.
        # `--wfail` keeps the build warning-free, so toolchain bumps surface
        # deprecations immediately instead of letting them pile up.
        ci-lean = assert pins-agree;
          pkgs.writeShellApplication {
            name = "bijou-ci-lean";
            runtimeInputs = [pkgs.elan pkgs.git];
            text = ''
              set -x
              cd "$(git rev-parse --show-toplevel)/lean"
              lake build --wfail
            '';
          };

        # Translate the Rust to Lean for the refinement proofs in
        # lean-aeneas/: Charon extracts the crate (LLBC), Aeneas emits a pure
        # Lean model of it. Regenerate after changing the translated
        # functions; the output is committed so proofs are reviewable.
        aeneas-translate = pkgs.writeShellApplication {
          name = "bijou-aeneas-translate";
          runtimeInputs = [aeneas-bin charon pkgs.coreutils pkgs.git pkgs.stdenv.cc];
          text = ''
            ${link-env}
            set -x
            cd "$(git rev-parse --show-toplevel)"
            tmp="$(mktemp -d)"
            trap 'rm -rf "$tmp"' EXIT
            export CARGO_TARGET_DIR="$PWD/target/charon"
            # Extraction roots: Charon translates these and what they call,
            # nothing else. The rest of the crate (DecodeIter, decode_all,
            # the Encoded* trait impls, Arbitrary) is out of scope, and much
            # of it Aeneas would leave opaque (trusted) or fail on.
            roots=()
            for width in i32 i64 i128 u32 u64 u128; do
              for fn in decode encode encoded_bytes encoded_len; do
                roots+=(--start-from "crate::$width::$fn")
              done
              ty="Encoded''${width^^}"
              roots+=(--start-from "crate::$width::$ty::as_slice")
            done
            (
              cd bijoux
              charon cargo --preset=aeneas \
                --dest-file "$tmp/bijoux.llbc" \
                "''${roots[@]}" \
                -- --lib --no-default-features --features i32,i64,i128,u32,u64,u128
            )
            # Generate into $tmp, then sync: the committed files are never
            # missing mid-run (a concurrent `git add -A` would stage them as
            # deleted). Everything Aeneas emits is copied, and generated files
            # it no longer emits are removed, so the staleness check in
            # ci-aeneas sees any change. FunsExternal.lean (hand-written models
            # of the core/alloc functions Aeneas leaves opaque) is ours.
            aeneas -backend lean -namespace BijouRust -split-files \
              -dest "$tmp/out" -subdir BijouAeneas/Generated \
              "$tmp/bijoux.llbc" "$@"
            new="$tmp/out/BijouAeneas/Generated"
            gen=lean-aeneas/BijouAeneas/Generated
            for f in "$gen"/*.lean; do
              b="$(basename "$f")"
              if [ "$b" != FunsExternal.lean ] && [ ! -e "$new/$b" ]; then rm "$f"; fi
            done
            for f in "$new"/*.lean; do
              if [ "$(basename "$f")" != FunsExternal.lean ]; then cp "$f" "$gen/"; fi
            done
          '';
        };

        # The refinement proofs in lean-aeneas/ are about the *committed*
        # translation. Re-translate and fail if that no longer matches the
        # current Rust, then check the proofs. Not in `ci-all`: needs Charon
        # and a Mathlib build (~7 GB, fetched prebuilt by `lake exe cache get`).
        ci-aeneas = pkgs.writeShellApplication {
          name = "bijou-ci-aeneas";
          runtimeInputs = [aeneas-translate pkgs.elan pkgs.git pkgs.python3];
          text = ''
            set -x
            cd "$(git rev-parse --show-toplevel)"
            bijou-aeneas-translate
            gen=lean-aeneas/BijouAeneas/Generated
            # Against the index (so staged-but-uncommitted output counts as
            # up to date), plus any new file the translation added. Hunks that
            # only move the `Source: '<file>', lines …` position comments are
            # ignored, so editing Rust docs doesn't need Charon to fix CI.
            if ! git diff --exit-code -I "Source: 'bijoux/src/" -- "$gen" ||
              [ -n "$(git ls-files --others --exclude-standard -- "$gen")" ]; then
              echo "$gen is stale: run 'nix run .#aeneas-translate' and commit the result" >&2
              exit 1
            fi
            # The 32/128-bit proofs (and every width's decode_spec) are
            # generated; they must match what the generator produces now.
            ref=lean-aeneas/BijouAeneas/Refinement
            python3 lean-aeneas/scripts/gen_widths.py "$ref" > /dev/null
            if ! git diff --exit-code -- "$ref"; then
              echo "$ref is out of date: run lean-aeneas/scripts/gen_widths.py and commit the result" >&2
              exit 1
            fi
            cd lean-aeneas
            lake exe cache get
            lake build --wfail
          '';
        };

        # Mutation testing (cargo-mutants; config in .cargo/mutants.toml).
        # Not in the `ci` aggregate: a full run takes far longer than the
        # rest of CI. Hosted CI scopes it to the PR's changed code with
        # `--in-diff` and does the full sweep nightly (mutants.yml).
        ci-mutants = pkgs.writeShellApplication {
          name = "bijou-ci-mutants";
          runtimeInputs = [rust-toolchain pkgs.cargo-mutants pkgs.stdenv.cc];
          text = ''
            ${link-env}
            set -x
            # No args: full workspace (slow, deliberate).
            # CI: bijou-ci-mutants --in-diff pr.diff
            #
            # Exit 3 means only timeouts: every mutant was detected, some by
            # making the tests stop terminating. Missed mutants exit 2, which
            # takes precedence, and still fail the job.
            #
            # A fixed iteration count gives every mutant the same number of
            # generated inputs per bolero harness.
            export BOLERO_RANDOM_ITERATIONS="''${BOLERO_RANDOM_ITERATIONS:-1000}"
            # Each test thread gets its own malloc arena; fewer threads keeps
            # per-mutant memory (and the address-space cap below) sane.
            export RUST_TEST_THREADS="''${RUST_TEST_THREADS:-4}"
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              # Some mutants turn a bounded loop into one that allocates
              # without bound (seen: 8–12 GB per test binary). That exhausts
              # RAM long before the per-mutant timeout, and the OOM killer /
              # systemd limiter takes out everything else with it. Capping
              # each process's address space makes the runaway test fail on
              # its own, which counts as caught. macOS ignores RLIMIT_AS.
              ulimit -v "$(( ''${BIJOU_MUTANTS_MEM_GB:-4} * 1024 * 1024 ))"
            ''}
            cargo mutants --workspace "$@" || [ $? -eq 3 ]
          '';
        };

        ci-all = pkgs.writeShellApplication {
          name = "bijou-ci";
          runtimeInputs = pkgs.lib.attrValues ci-checks ++ [ci-lean ci-wasm-node];
          text =
            pkgs.lib.concatMapStringsSep "\n"
            (check: "bijou-${check}")
            (builtins.attrNames ci-checks ++ ["ci-lean" "ci-wasm-node"])
            + ''

              echo
              echo "NOT RUN here (hosted CI runs them separately):"
              echo "  test:js             # wasm-bodge package in Node + browsers (test-wasm.yml)"
              echo "  nix run .#ci-mutants  # mutation testing (CI: PR diff + nightly full)"
              echo "  nix run .#ci-aeneas   # Rust-refines-model proofs (Charon + Mathlib)"
            '';
        };

        cargoPath = "${rust-toolchain}/bin/cargo";

        format-pkgs = with pkgs; [
          alejandra
          nixpkgs-fmt
          taplo
        ];

        # `cargo-release` comes from unstable: the 26.05 backport of 1.1.7
        # requires rustc >= 1.97, but 26.05 ships rustc 1.95.
        cargo-installs = with pkgs; [
          cargo-audit
          cargo-component
          cargo-criterion
          cargo-deny
          cargo-expand
          cargo-flamegraph
          cargo-hack
          cargo-mutants
          cargo-nextest
          cargo-outdated
          cargo-semver-checks
          cargo-sort
          cargo-udeps
          cargo-watch
          twiggy
          typos
          unstable.cargo-release
          wasm-bindgen-cli
          wasm-tools
        ];

        # `xdg-utils` lacks `meta.mainProgram`, which makes `getExe`
        # (used by the command-utils modules below) emit a deprecation
        # warning. Annotate it so `getExe` resolves `xdg-open` cleanly.
        xdg-open =
          pkgs.xdg-utils
          // {
            meta = (pkgs.xdg-utils.meta or {}) // {mainProgram = "xdg-open";};
          };

        # Built-in command modules from nix-command-utils
        rust = command-utils.rust.${system};
        wasm = command-utils.wasm.${system};
        cmd = command-utils.cmd.${system};

        # Python environment for benchmark chart generation (analyze.py)
        bench-charts-python = pkgs.python3.withPackages (ps: [
          ps.matplotlib
          ps.numpy
          ps.pandas
          ps.plotly
          ps.seaborn
        ]);

        bench-charts = pkgs.writeShellScriptBin "bench-charts" ''
          exec "${bench-charts-python}/bin/python3" "$WORKSPACE_ROOT/bijoux/charts/analyze.py" "$@"
        '';

        size-charts = pkgs.writeShellScriptBin "size-charts" ''
          exec "${bench-charts-python}/bin/python3" "$WORKSPACE_ROOT/bijoux/charts/size_charts.py" "$@"
        '';

        # Project-specific commands
        projectCommands = {
          "bench:shootout" = cmd "Run the bijou64 criterion shootout benchmark" ''
            ${cargoPath} bench --package bijoux --bench shootout
          '';

          "bench:shootout:signed" = cmd "Run the signed-format criterion shootout benchmark" ''
            ${cargoPath} bench --package bijoux --bench signed_shootout
          '';

          "bench:gungraun" = cmd "Run the bijou64 gungraun instruction-count benchmark" ''
            ${cargoPath} bench --package bijoux --bench gungraun_shootout
          '';

          "bench:charts" = cmd "Generate benchmark comparison charts (requires bench results in target/criterion)" ''
            ${bench-charts}/bin/bench-charts "$@"
          '';

          "size:charts" = cmd "Generate encoded-size comparison charts (arch-independent, no benchmarks needed)" ''
            ${size-charts}/bin/size-charts "$@"
          '';

          "test:props" =
            cmd "Run property tests with 1M iterations"
            ''exec ${ci-checks.ci-bolero}/bin/bijou-ci-bolero "$@"'';

          "test:props:intense" = cmd "Run property tests with 100M iterations (~10 min)" ''
            set -e
            echo "Running property tests with 100,000,000 iterations each..."
            export BOLERO_RANDOM_ITERATIONS=100000000
            ${cargoPath} test --release --workspace --features bolero --lib tests::property -- --nocapture
          '';

          "test:no_std" =
            cmd "Check no_std builds for every bijoux width (host + thumbv6m)"
            ''exec ${ci-checks.ci-no-std}/bin/bijou-ci-no-std "$@"'';

          "check:wasm" =
            cmd "Check the workspace builds for wasm32-unknown-unknown"
            ''exec ${ci-checks.ci-wasm}/bin/bijou-ci-wasm "$@"'';

          "check:msrv" =
            cmd "Check bijoux builds on the MSRV (${msrv})"
            ''exec ${ci-checks.ci-msrv}/bin/bijou-ci-msrv "$@"'';

          "test:mutants" =
            cmd "Run mutation testing (slow; pass --in-diff <file> to scope)"
            ''exec ${ci-mutants}/bin/bijou-ci-mutants "$@"'';

          "bodge" = cmd "Build bijoux_wasm into a universal NPM package via wasm-bodge (release + /debug variants)" ''
            set -e
            ${pkgs.coreutils}/bin/rm -rf "$WORKSPACE_ROOT/bijoux_wasm/dist"
            echo "===> wasm-bodge build bijoux_wasm..."
            # panic=unwind (the wasm-bodge default): recoverable Rust panics
            # surface as catchable JS PanicErrors instead of aborting the Wasm
            # instance. Requires nightly + rust-src (std is rebuilt via
            # -Zbuild-std) and wasm-bindgen >= 0.2.127. wasm-bodge invokes
            # `cargo +nightly`, so route it through the cargo shim.
            PATH="${cargo-nightly-shim}/bin:${wasm-opt-shim}/bin:$PATH" \
            ${wasm-bodge}/bin/wasm-bodge build \
              --crate-path "$WORKSPACE_ROOT/bijoux_wasm" \
              --package-json "$WORKSPACE_ROOT/bijoux_wasm/package.json" \
              --out-dir "$WORKSPACE_ROOT/bijoux_wasm/dist" \
              --debug-profile wasm-debug
            echo ""
            echo "✓ bijoux_wasm built — output in bijoux_wasm/dist/"
          '';

          "wasm:sizes" = cmd "Print wasm bundle sizes for all bijou wasm packages" ''
            set -e

            format_size() {
              local size=$1
              if [ "$size" -gt 1048576 ]; then
                echo "$(echo "scale=2; $size / 1048576" | ${pkgs.bc}/bin/bc) MB"
              elif [ "$size" -gt 1024 ]; then
                echo "$(echo "scale=2; $size / 1024" | ${pkgs.bc}/bin/bc) KB"
              else
                echo "$size B"
              fi
            }

            rows=""
            for dir in bijoux_wasm; do
              wasm_file=$(ls "$WORKSPACE_ROOT/$dir/dist/wasm_bindgen/web/"*_bg.wasm 2>/dev/null | head -1)
              if [ -n "$wasm_file" ] && [ -f "$wasm_file" ]; then
                name=$(basename "$dir")
                raw_size=$(${pkgs.coreutils}/bin/stat -c%s "$wasm_file" 2>/dev/null || echo "0")
                gzip_size=$(${pkgs.gzip}/bin/gzip -c "$wasm_file" | ${pkgs.coreutils}/bin/wc -c)
                raw_fmt=$(format_size "$raw_size")
                gz_fmt=$(format_size "$gzip_size")
                rows="$rows$name|$raw_fmt|$gz_fmt\n"
              fi
            done

            echo ""
            echo "┌───────────────────────────────────┬────────────┬────────────┐"
            echo "│ Package                           │        Raw │    Gzipped │"
            echo "├───────────────────────────────────┼────────────┼────────────┤"

            printf "$rows" | while IFS='|' read -r name raw gz; do
              printf "│ %-33s │ %10s │ %10s │\n" "$name" "$raw" "$gz"
            done

            echo "└───────────────────────────────────┴────────────┴────────────┘"
            echo ""
          '';

          "test:js" = cmd "Run bijoux_wasm JS-package tests in Node + browsers (rebuilds dist via bodge)" ''
            set -e
            "test:js:node"
            "test:js:browser"
          '';

          "test:js:node" = cmd "Run bijoux_wasm JS-package tests in Node.js via Mocha (rebuilds dist via bodge)" ''
            set -e
            bodge
            cd "$WORKSPACE_ROOT/bijoux_wasm"
            if [ ! -d node_modules ]; then
              ${pkgs.pnpm}/bin/pnpm install --frozen-lockfile
            fi
            ${pkgs.pnpm}/bin/pnpm run test:js:node
          '';

          "test:js:browser" = cmd "Run bijoux_wasm JS-package Playwright tests across browsers (rebuilds dist via bodge)" ''
            set -e
            bodge
            cd "$WORKSPACE_ROOT/bijoux_wasm"
            # `--frozen-lockfile` mirrors what CI does and makes sure the
            # local run cannot silently mutate `pnpm-lock.yaml` if it
            # disagrees with `package.json`. To intentionally update the
            # lockfile, run `pnpm install` directly (without this wrapper).
            if [ ! -d node_modules ]; then
              ${pkgs.pnpm}/bin/pnpm install --frozen-lockfile
            fi
            ${pkgs.pnpm}/bin/pnpm exec playwright test
          '';

          "test:js:browser:report" = cmd "Open the most recent Playwright HTML report" ''
            cd "$WORKSPACE_ROOT/bijoux_wasm"
            if [ ! -d node_modules ]; then
              ${pkgs.pnpm}/bin/pnpm install --frozen-lockfile
            fi
            ${pkgs.pnpm}/bin/pnpm exec playwright show-report
          '';

          "proofs" =
            cmd "Check the Lean 4 format proofs in lean/"
            ''exec ${ci-lean}/bin/bijou-ci-lean "$@"'';

          "ci" =
            cmd "Run the full CI suite (same checks as hosted CI)"
            ''exec ${ci-all}/bin/bijou-ci "$@"'';

          "ci:quick" = cmd "Run quick CI checks (fmt, clippy, test)" ''
            set -e
            ${ci-checks.ci-fmt}/bin/bijou-ci-fmt
            ${ci-checks.ci-clippy}/bin/bijou-ci-clippy
            ${ci-checks.ci-test}/bin/bijou-ci-test
          '';
        };

        # Only in `devShells.proofs`: these close over Charon + Aeneas.
        proofCommands = {
          "proofs:aeneas" =
            cmd "Check the Rust-refines-model proofs in lean-aeneas/ (re-translates; Charon + Mathlib)"
            ''exec ${ci-aeneas}/bin/bijou-ci-aeneas "$@"'';

          "aeneas:translate" =
            cmd "Re-translate bijoux to Lean (Charon + Aeneas) into lean-aeneas/BijouAeneas/Generated/"
            ''exec ${aeneas-translate}/bin/bijou-aeneas-translate "$@"'';
        };

        menu-modules = [
          # Rust commands
          (rust.audit {cargo-audit = pkgs.cargo-audit;})
          (rust.build {cargo = pkgs.cargo;})
          (rust.test {
            cargo = pkgs.cargo;
            cargo-watch = pkgs.cargo-watch;
          })
          (rust.lint {cargo = pkgs.cargo;})
          (rust.fmt {cargo = pkgs.cargo;})
          (rust.doc {cargo = pkgs.cargo;})
          (rust.bench {
            cargo = pkgs.cargo;
            cargo-criterion = pkgs.cargo-criterion;
            inherit xdg-open;
          })
          (rust.watch {cargo-watch = pkgs.cargo-watch;})
          (rust.semver {cargo-semver-checks = pkgs.cargo-semver-checks;})

          # Wasm commands need to point at a `[package]`-manifest crate
          # (the workspace root has only `[workspace]` and would make
          # wasm-pack bail out with "missing field `package`").
          (wasm.build {
            wasm-pack = pkgs.wasm-pack;
            path = "bijoux_wasm";
          })
          (wasm.release {
            wasm-pack = pkgs.wasm-pack;
            gzip = pkgs.gzip;
            path = "bijoux_wasm";
          })
          (wasm.test {
            wasm-pack = pkgs.wasm-pack;
            path = "bijoux_wasm";
            features = "";
          })
          (wasm.doc {
            cargo = pkgs.cargo;
            inherit xdg-open;
          })

          (command-utils.asModule.${system} projectCommands)
        ];

        command_menu = command-utils.commands.${system} menu-modules;
        proofs_command_menu =
          command-utils.commands.${system}
          (menu-modules ++ [(command-utils.asModule.${system} proofCommands)]);
      in {
        packages =
          {inherit gungraun-runner wasm-bodge;}
          // pkgs.lib.optionalAttrs has-aeneas {
            aeneas = aeneas-bin;
            inherit charon;
          };

        devShells = let
          shells = rec {
            default = pkgs.mkShell {
              name = "bijou_shell";

              nativeBuildInputs =
                # `command_menu` is itself a list of derivations, so splice it
                # in with `++` rather than nesting it as a single element
                # (a nested list in `nativeBuildInputs` is deprecated).
                command_menu
                ++ [
                  rust-toolchain
                  nightly-rustfmt

                  pkgs.binaryen
                  pkgs.elan # Lean toolchains for lean/ and lean-aeneas/ (reads lean-toolchain)
                  pkgs.esbuild # wasm-bodge spawns esbuild as a subprocess for bundling
                  pkgs.gnuplot
                  pkgs.http-server
                  pkgs.nodejs
                  pkgs.pnpm
                  pkgs.playwright-driver
                  pkgs.playwright-driver.browsers
                  pkgs.rust-analyzer
                  pkgs.valgrind # required by gungraun
                  pkgs.wasm-pack
                  gungraun-runner
                  wasm-bodge
                ]
                ++ format-pkgs
                ++ cargo-installs
                ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                  pkgs.libiconv
                ]
                ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
                  pkgs.clang
                  pkgs.llvmPackages.libclang
                  pkgs.pkg-config
                ];

              shellHook = ''
                unset SOURCE_DATE_EPOCH
                export WORKSPACE_ROOT="$(pwd)"
                export RUSTFMT="${nightly-rustfmt}/bin/rustfmt"

                # Point Playwright at Nix-provided browsers instead of letting
                # it download its own (which fails on NixOS due to dynamic
                # linking, and is wasteful elsewhere).
                export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
                export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
                export PLAYWRIGHT_NODEJS_PATH="${pkgs.nodejs}/bin/node"

                menu
              '';
            };

            # Default shell plus Charon + Aeneas, for working on lean-aeneas/
            # (`nix develop .#proofs`). Kept out of the default shell: a large
            # closure (OCaml, a pinned rustc nightly) most contributors never need.
            proofs = default.overrideAttrs (old: {
              name = "bijou_proofs_shell";
              nativeBuildInputs =
                proofs_command_menu
                ++ [aeneas-bin charon]
                ++ pkgs.lib.subtractLists command_menu old.nativeBuildInputs;
            });
          };
        in
          if has-aeneas
          then shells
          else removeAttrs shells ["proofs"];

        apps =
          {
            bench-charts = {
              type = "app";
              program = "${bench-charts}/bin/bench-charts";
            };

            size-charts = {
              type = "app";
              program = "${size-charts}/bin/size-charts";
            };
          }
          // pkgs.lib.mapAttrs (name: check: {
            type = "app";
            program = "${check}/bin/bijou-${name}";
          })
          (ci-checks
            // {
              ci = ci-all;
              inherit ci-lean ci-mutants ci-wasm-node;
            }
            // pkgs.lib.optionalAttrs has-aeneas {
              inherit aeneas-translate ci-aeneas;
            });

        formatter = pkgs.alejandra;
      }
    );
}
