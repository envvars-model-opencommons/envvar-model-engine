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
          exts.open-vsx.davidanson.vscode-markdownlint # README, ARCHITECTURE, CHANGELOG
          exts.open-vsx.redhat.vscode-yaml # the GitHub workflows
          exts.open-vsx.jnoortheen.nix-ide # this file
          exts.open-vsx.mkhl.direnv # picks up .envrc in-editor
        ];
      };

      # Rewritten on every launch so the store paths below stay fresh.
      settings = pkgs.writeText "settings.json" (builtins.toJSON {
        "telemetry.telemetryLevel" = "off";
        "update.mode" = "none";
        "extensions.autoCheckUpdates" = false;
        "security.workspace.trust.enabled" = false;

        # --- Rust ---
        "rust-analyzer.server.path" = "${pkgs.rust-analyzer}/bin/rust-analyzer";
        "rust-analyzer.rustfmt.overrideCommand" = [ "${pkgs.rustfmt}/bin/rustfmt" ];
        # CI runs `clippy -D warnings`; surface the same lints while typing rather
        # than discovering them on push.
        "rust-analyzer.check.command" = "clippy";
        "rust-analyzer.check.extraArgs" = [ "--" "-D" "warnings" ];
        # The `contract` feature gates the schema emitter and its tests. Without
        # this, contract.rs and tests/schema.rs are greyed out and unanalysed.
        "rust-analyzer.cargo.features" = "all";
        "rust-analyzer.check.features" = "all";
        # Keep the analyser out of ./target so it does not fight the terminal for
        # the build lock.
        "rust-analyzer.cargo.targetDir" = true;

        "[rust]" = {
          "editor.defaultFormatter" = "rust-lang.rust-analyzer";
          "editor.formatOnSave" = true;
        };

        # --- Nix (this flake) ---
        "nix.enableLanguageServer" = true;
        "nix.serverPath" = "${pkgs.nil}/bin/nil";
        "nix.serverSettings" = {
          nil.formatting.command = [ "${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt" ];
        };
        "[nix]" = {
          "editor.defaultFormatter" = "jnoortheen.nix-ide";
          "editor.formatOnSave" = true;
        };

        # --- generated artefacts are read-only by convention ---
        # schema/ and api/ are emitted by `argenv schema`; a test fails if a
        # hand edit makes them disagree with the model.
        "files.readonlyInclude" = {
          "schema/**" = true;
          "api/v1/**" = true;
        };

        "files.associations" = {
          ".envrc" = "shellscript";
        };
        "files.exclude" = {
          "**/target" = true;
          "**/.direnv" = true;
          "**/.ide" = true;
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
        ];

        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        shellHook = ''
          echo "🔌 argenv dev shell — 'code .' for the isolated editor"
          echo "   cargo test --workspace --all-features   # 79 tests"
          echo "   cargo run -p argenv-cli -- schema -o schema/argenv-contract.v1.schema.json"
        '';
      };
    };
}
