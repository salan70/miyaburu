{
  description = "みやぶる — Rust製MarkdownプレビューCLIの開発環境";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      # 対象は現在のMac（Apple Silicon）とLinux。
      systems = [ "aarch64-darwin" "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f (import nixpkgs { inherit system; }));
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    in
    {
      # `nix profile add github:salan70/miyaburu` と `nix run` で使う実行ファイル。
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          # ビルドとテストに使うものだけを入力にし、文書だけの変更では作り直さない。
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./src
              ./assets
              ./tests
            ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          meta = {
            description = cargoToml.package.description;
            homepage = cargoToml.package.repository;
            mainProgram = "miyaburu";
          };
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            # DocBridgeはnpmレジストリで配布されるため、nixpkgsでは固定できない。
            # 版は .docbridge-version で決め、tools/docbridge.sh が pnpm dlx で呼び出す。
            nodejs_22
            pnpm
            just
            actionlint
            shellcheck
          ];
        };
      });
    };
}
