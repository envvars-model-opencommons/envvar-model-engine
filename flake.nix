{
  description = "Rust dev shell with a fully isolated, pinned VSCodium";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # your local bacon-ls fork with the package=null fix.
    bacon-ls-fork.url = "git+file:///home/_/projects/bacon-ls";
    bacon-ls-fork.flake = false;

    nix-vscode-extensions.url = "github:nix-community/nix-vscode-extensions";
    nix-vscode-extensions.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, bacon-ls-fork, nix-vscode-extensions }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      exts = nix-vscode-extensions.extensions.${system};

      # Build bacon-ls from your fork. No hashes: src + Cargo.lock come from the
      # input, and the 0.29.0 lockfile has no git deps, so nothing else is needed.
      baconLs = pkgs.rustPlatform.buildRustPackage {
        pname = "bacon-ls";
        version = "0.29.0-fork";
        src = bacon-ls-fork;
        cargoLock.lockFile = "${bacon-ls-fork}/Cargo.lock";
        doCheck = false;
      };

      ide = pkgs.vscode-with-extensions.override {
        vscode = pkgs.vscodium;
        vscodeExtensions = [
          exts.open-vsx.rust-lang.rust-analyzer
          exts.open-vsx.matteobigoi.bacon-ls-vscode
          exts.open-vsx.tamasfe.even-better-toml
        ];
      };

      # Rewritten on every launch so the bacon-ls store path stays fresh.
      settings = pkgs.writeText "settings.json" (builtins.toJSON {
        "rust-analyzer.checkOnSave" = false;
        "rust-analyzer.diagnostics.enable" = false;
        "rust-analyzer.server.path" = "${pkgs.rust-analyzer}/bin/rust-analyzer";
        "bacon-ls.path" = "${baconLs}/bin/bacon-ls";   # your patched fork
        "bacon-ls.logLevel" = "debug";                  # so ./bacon-ls.log confirms the fix
        "telemetry.telemetryLevel" = "off";
        "update.mode" = "none";
      });

      launch = ''
        set -eu
        user="$PWD/.ide/user-data"
        mkdir -p "$user/User"
        install -m600 ${settings} "$user/User/settings.json"
        exec ${ide}/bin/codium --user-data-dir="$user" "$@"
      '';
      ideLauncher = pkgs.writeShellScriptBin "code" launch;
    in {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.openssl ];

        packages = with pkgs; [
          rustc cargo rustfmt clippy rust-analyzer
          bacon baconLs
          ideLauncher
        ];

        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        shellHook = ''echo "🦀 rust dev shell — run 'code .' for the isolated editor"'';
      };
    };
}