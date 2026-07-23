{
  description = "argenv dev shell with a fully isolated, pinned VSCodium";

  # This flake lives in ./dev on purpose. The repository root deliberately has no
  # flake.nix: argenv is published to crates.io, not as a Nix module, and a root
  # flake would invite the two to be confused. Nothing here builds the project —
  # cargo does that. This provides the toolchain and the editor.

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    nix-vscode-extensions.url = "github:nix-community/nix-vscode-extensions";
    nix-vscode-extensions.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, nix-vscode-extensions }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      exts = nix-vscode-extensions.extensions.${system};

      # The conformance job validates the emitted schema with a third-party
      # engine rather than our own code. Devs get the same Python here, so the
      # check is runnable locally instead of only in CI.
      python = pkgs.python3.withPackages (ps: [ ps.jsonschema ]);

      ide = pkgs.vscode-with-extensions.override {
        vscode = pkgs.vscodium;
        vscodeExtensions = [
          exts.open-vsx.rust-lang.rust-analyzer # Rust language server
          exts.open-vsx.tamasfe.even-better-toml # Cargo.toml
          exts.open-vsx.serayuzgur.crates # dependency versions inline
          exts.open-vsx.ryanluker.vscode-coverage-gutters # coverage in the margin
          exts.open-vsx.davidanson.vscode-markdownlint # README, ARCHITECTURE, CHANGELOG
          exts.open-vsx.redhat.vscode-yaml # the GitHub workflows
          exts.open-vsx.jnoortheen.nix-ide # this file
          exts.open-vsx.mkhl.direnv # picks up .envrc in-editor

          # CodeLLDB, for the Debug lens and .vscode/launch.json.
          #
          # Taken from nixpkgs rather than the marketplace on purpose: this
          # extension ships a prebuilt debug adapter, and nixpkgs patches it for
          # NixOS while a scraped marketplace build would not run. If debugging
          # ever misbehaves, this is the one line to remove — everything else,
          # including the test buttons and the Testing panel, works without it.
          pkgs.vscode-extensions.vadimcn.vscode-lldb
        ];
      };

      # Only what cannot live in .vscode/settings.json: store paths, which are
      # machine-specific, and personal preferences. Everything project-level is
      # committed in .vscode/settings.json so contributors without Nix get it
      # too — and is deliberately absent here, because workspace settings
      # override these and duplication would mean two copies that can drift.
      settings = pkgs.writeText "settings.json" (builtins.toJSON {
        "telemetry.telemetryLevel" = "off";
        "update.mode" = "none";
        "extensions.autoCheckUpdates" = false;
        "security.workspace.trust.enabled" = false;

        "rust-analyzer.server.path" = "${pkgs.rust-analyzer}/bin/rust-analyzer";
        "rust-analyzer.rustfmt.overrideCommand" = [ "${pkgs.rustfmt}/bin/rustfmt" ];

        "nix.enableLanguageServer" = true;
        "nix.serverPath" = "${pkgs.nil}/bin/nil";
        "nix.serverSettings" = {
          nil.formatting.command = [ "${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt" ];
        };
      });

      # Isolated launcher: a per-project user-data directory, so this editor
      # shares no state with any global VSCodium and vice versa.
      launch = ''
        set -eu
        user="$PWD/.ide/user-data"
        mkdir -p "$user/User"
        install -m600 ${settings} "$user/User/settings.json"
        exec ${ide}/bin/codium --user-data-dir="$user" "$@"
      '';
      # One command for coverage, so the editor and the terminal agree on where
      # the report lands.
      coverage = pkgs.writeShellScriptBin "coverage" ''
        set -eu
        mkdir -p target/coverage
        cargo llvm-cov --workspace --all-features \
          --lcov --output-path target/coverage/lcov.info "$@"
        echo
        echo "lcov written to target/coverage/lcov.info"
        echo "In the editor: press Watch in the Coverage Gutters status bar."
        echo "Browsable report: cargo llvm-cov --workspace --all-features --html --open"
      '';

      code = pkgs.writeShellScriptBin "code" launch;
      codeDev = pkgs.writeShellScriptBin "code-dev" launch;
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          # Rust: the same set CI uses
          rustc
          cargo
          rustfmt
          clippy
          rust-analyzer
          cargo-llvm-cov

          # Contract work: validate the emitted schema with a foreign engine,
          # and read the generated JSON without squinting.
          python
          jq

          # Nix tooling for this flake
          nil
          nixpkgs-fmt

          git

          # The isolated editor — two names, same IDE
          code
          codeDev
          coverage
        ];

        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        # cargo-llvm-cov shells out to the LLVM tools. Nixpkgs rustc does not
        # ship them where it looks, so point at them explicitly — without this,
        # coverage fails with a confusing "llvm-profdata not found".
        LLVM_COV = "${pkgs.llvmPackages.llvm}/bin/llvm-cov";
        LLVM_PROFDATA = "${pkgs.llvmPackages.llvm}/bin/llvm-profdata";

        shellHook = ''
          echo "🔌 argenv dev shell — 'code .' for the isolated editor"
          echo "   cargo test --workspace --all-features   # 79 tests"
          echo "   coverage                                # lcov for the editor margin"
          echo "   cargo run -p argenv-cli -- schema -o schema/argenv-contract.v1.schema.json"
        '';
      };
    };
}
