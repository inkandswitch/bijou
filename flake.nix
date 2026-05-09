{
  description = "bijou — bijective variable-length integer encodings";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-25.11";
    nixos-unstable.url = "nixpkgs/nixos-unstable-small";

    command-utils.url = "git+https://codeberg.org/expede/nix-command-utils";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    wasm-bodge-src = {
      # Tracks the upstream `main` branch. wasm-bodge moves quickly enough
      # that being a release or two behind is genuinely costly (see e.g. the
      # debug/slim and initSync({module}) fixes that landed post-v0.2.3).
      url = "github:alexjg/wasm-bodge/main";
      flake = false;
    };
  };

  outputs = {
    self,
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

        rustVersion = "1.90.0";

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
          extensions = [ "rustfmt" ];
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
          version = wasm-bodge-src.shortRev;
          src = wasm-bodge-src;
          cargoHash = "sha256-akp4r8C4MWGqTbqr40jHdHuzqx6ZKcr4rFynarPsZWI=";
          nativeBuildInputs = [ unstable.cargo-auditable ];
          doCheck = false; # tests require npm/puppeteer infrastructure
        };

        # gungraun-runner: required binary harness for gungraun (formerly
        # iai-callgrind) instruction-count benchmarks. Not yet in nixpkgs;
        # we build it from crates.io. Version must match the `gungraun`
        # workspace dep in `bijou64/Cargo.toml`.
        gungraun-runner = pinnedRustPlatform.buildRustPackage rec {
          pname = "gungraun-runner";
          version = "0.18.2";

          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-DiJq9TZCZdWKSstIyMjkLuxaYXua0WKD2AVbEIxM590=";
          };

          cargoHash = "sha256-eb9U1MgCg7MpwzS2RnFXMWdPitweKMMty0n3SC0F6+I=";

          # Tests require a full benchmark execution loop with valgrind.
          # We're shipping just the binary harness.
          doCheck = false;
        };

        # wasm-bindgen-cli 0.2.118 (not yet in nixpkgs)
        wasm-bindgen-cli_0_2_118 = unstable.buildWasmBindgenCli rec {
          src = unstable.fetchCrate {
            pname = "wasm-bindgen-cli";
            version = "0.2.118";
            hash = "sha256-ve783oYH0TGv8Z8lIPdGjItzeLDQLOT5uv/jbFOlZpI=";
          };

          cargoDeps = unstable.rustPlatform.fetchCargoVendor {
            inherit src;
            inherit (src) pname version;
            hash = "sha256-EYDfuBlH3zmTxACBL+sjicRna84CvoesKSQVcYiG9P0=";
          };
        };

        format-pkgs = with pkgs; [
          alejandra
          nixpkgs-fmt
          taplo
        ];

        cargo-installs = with pkgs; [
          cargo-component
          cargo-criterion
          cargo-deny
          cargo-expand
          cargo-flamegraph
          cargo-nextest
          cargo-outdated
          cargo-release
          cargo-sort
          cargo-udeps
          cargo-watch
          twiggy
          wasm-bindgen-cli_0_2_118
          wasm-tools
        ];

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
          exec "${bench-charts-python}/bin/python3" "$WORKSPACE_ROOT/bijou64/charts/analyze.py" "$@"
        '';

        # Project-specific commands
        projectCommands = {
          "bench:shootout" = cmd "Run the bijou64 criterion shootout benchmark" ''
            ${pkgs.cargo}/bin/cargo bench --package bijou64 --bench shootout
          '';

          "bench:gungraun" = cmd "Run the bijou64 gungraun instruction-count benchmark" ''
            ${pkgs.cargo}/bin/cargo bench --package bijou64 --bench gungraun_shootout
          '';

          "bench:charts" = cmd "Generate benchmark comparison charts (requires bench results in target/criterion)" ''
            ${bench-charts}/bin/bench-charts "$@"
          '';

          "test:props" = cmd "Run property tests with 1M iterations" ''
            set -e
            echo "Running property tests with 1,000,000 iterations each..."
            export BOLERO_RANDOM_ITERATIONS=1000000
            ${pkgs.cargo}/bin/cargo test --release --workspace --features bolero --lib tests::property -- --nocapture
            echo ""
            echo "✓ All property tests passed"
          '';

          "test:props:intense" = cmd "Run property tests with 100M iterations (~10 min)" ''
            set -e
            echo "Running property tests with 100,000,000 iterations each..."
            export BOLERO_RANDOM_ITERATIONS=100000000
            ${pkgs.cargo}/bin/cargo test --release --workspace --features bolero --lib tests::property -- --nocapture
          '';

          "test:no_std" = cmd "Check no_std build for bijou64" ''
            set -e
            ${pkgs.cargo}/bin/cargo check --package bijou64 --no-default-features -v
            echo ""
            echo "✓ no_std check passed"
          '';

          "check:wasm" = cmd "Check the workspace builds for wasm32-unknown-unknown" ''
            set -e
            ${pkgs.cargo}/bin/cargo check --workspace --target wasm32-unknown-unknown
            echo ""
            echo "✓ wasm32 check passed"
          '';

          "bodge" = cmd "Build bijou64_wasm into a universal NPM package via wasm-bodge" ''
            set -e
            ${pkgs.coreutils}/bin/rm -rf "$WORKSPACE_ROOT/bijou64_wasm/dist"
            echo "===> wasm-bodge build bijou64_wasm..."
            ${wasm-bodge}/bin/wasm-bodge build \
              --crate-path "$WORKSPACE_ROOT/bijou64_wasm" \
              --package-json "$WORKSPACE_ROOT/bijou64_wasm/package.json" \
              --out-dir "$WORKSPACE_ROOT/bijou64_wasm/dist"
            echo ""
            echo "✓ bijou64_wasm built — output in bijou64_wasm/dist/"
          '';

          # Rust integration tests on the wasm32 target. Run via Node.js
          # (the wasm32 ABI is identical across runtimes; cross-browser
          # behaviour is covered by Playwright at the JS-package level).
          "test:wasm:node" = cmd "Run bijou64_wasm Rust tests on wasm32 in Node.js" ''
            set -e
            ${pkgs.wasm-pack}/bin/wasm-pack test --node bijou64_wasm
          '';

          "test:e2e" = cmd "Run bijou64_wasm Playwright tests across browsers (rebuilds dist via bodge)" ''
            set -e
            bodge
            cd "$WORKSPACE_ROOT/bijou64_wasm"
            if [ ! -d node_modules ]; then
              ${pkgs.nodePackages.pnpm}/bin/pnpm install
            fi
            ${pkgs.nodePackages.pnpm}/bin/pnpm exec playwright test
          '';

          "test:e2e:report" = cmd "Open the most recent Playwright HTML report" ''
            cd "$WORKSPACE_ROOT/bijou64_wasm"
            ${pkgs.nodePackages.pnpm}/bin/pnpm exec playwright show-report
          '';

          "ci" = cmd "Run full CI suite (fmt, clippy, test, no_std, wasm32, wasm-pack test)" ''
            set -e

            echo "===> [1/6] Checking formatting..."
            ${pkgs.cargo}/bin/cargo fmt --check
            echo "✓ Formatting OK"
            echo ""

            echo "===> [2/6] Running Clippy..."
            ${pkgs.cargo}/bin/cargo clippy --workspace --all-targets --all-features -- -D warnings
            echo "✓ Clippy OK"
            echo ""

            echo "===> [3/6] Running host tests..."
            ${pkgs.cargo}/bin/cargo test --workspace --all-features
            echo "✓ Host tests OK"
            echo ""

            echo "===> [4/6] Checking no_std..."
            ${pkgs.cargo}/bin/cargo check --package bijou64 --no-default-features
            echo "✓ no_std OK"
            echo ""

            echo "===> [5/6] Checking wasm32 build..."
            ${pkgs.cargo}/bin/cargo check --workspace --target wasm32-unknown-unknown
            echo "✓ wasm32 OK"
            echo ""

            echo "===> [6/6] Running wasm-pack tests in Node.js..."
            ${pkgs.wasm-pack}/bin/wasm-pack test --node bijou64_wasm
            echo "✓ wasm-pack tests OK"
            echo ""

            echo "✓ All CI checks passed"
          '';
        };

        command_menu = command-utils.commands.${system} [
          # Rust commands
          (rust.build { cargo = pkgs.cargo; })
          (rust.test { cargo = pkgs.cargo; cargo-watch = pkgs.cargo-watch; })
          (rust.lint { cargo = pkgs.cargo; })
          (rust.fmt { cargo = pkgs.cargo; })
          (rust.doc { cargo = pkgs.cargo; })
          (rust.bench { cargo = pkgs.cargo; cargo-criterion = pkgs.cargo-criterion; xdg-open = pkgs.xdg-utils; })
          (rust.watch { cargo-watch = pkgs.cargo-watch; })

          # Wasm commands
          (wasm.build { wasm-pack = pkgs.wasm-pack; })
          (wasm.release { wasm-pack = pkgs.wasm-pack; gzip = pkgs.gzip; })
          (wasm.test { wasm-pack = pkgs.wasm-pack; features = ""; })
          (wasm.doc { cargo = pkgs.cargo; xdg-open = pkgs.xdg-utils; })

          { commands = projectCommands; packages = []; }
        ];

      in {
        apps.bench-charts = {
          type = "app";
          program = "${bench-charts}/bin/bench-charts";
        };

        packages = {
          inherit gungraun-runner wasm-bodge;
        };

        devShells.default = pkgs.mkShell {
          name = "bijou_shell";

          nativeBuildInputs =
            [
              command_menu
              rust-toolchain
              nightly-rustfmt

              pkgs.binaryen
              pkgs.gnuplot
              pkgs.http-server
              pkgs.nodejs
              pkgs.nodePackages.pnpm
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

        formatter = pkgs.alejandra;
      }
    );
}
