#!/bin/sh
# miyaburu を検証してユーザー領域へインストールします。

main() {
    set -eu

    repo='salan70/miyaburu'
    base="https://github.com/$repo"
    version=${MIYABURU_VERSION-}
    install_dir=${MIYABURU_INSTALL_DIR-"${HOME-}/.local/bin"}
    work=
    staged=

    cleanup() {
        if [ -n "$staged" ]; then rm -f "$staged"; fi
        if [ -n "$work" ]; then rm -rf "$work"; fi
    }
    trap cleanup 0
    trap 'exit 129' HUP
    trap 'exit 130' INT
    trap 'exit 143' TERM

    fail() { printf 'エラー: %s\n' "$*" >&2; exit 1; }

    case $install_dir in /*) ;; *) fail '導入先は絶対パスで指定してください。' ;; esac
    case $version in
        '')
            work=$(mktemp -d "${TMPDIR:-/tmp}/miyaburu-install.XXXXXX") || fail '一時領域を作成できません。'
            latest=$(curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location --output /dev/null --write-out '%{url_effective}' "$base/releases/latest") || fail '最新リリースを取得できません。'
            version=${latest##*/}
            ;;
        *) case $version in v*) ;; *) version="v$version" ;; esac ;;
    esac
    case $version in v[0-9]*.[0-9]*.[0-9]*) ;; *) fail '版指定が不正です（例: 0.1.0）。' ;; esac
    case $version in *[!v0-9.]*) fail '版指定が不正です（例: 0.1.0）。' ;; esac
    # 形式を厳密に v数字.数字.数字 に限定します。
    printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' || fail '版指定が不正です（例: 0.1.0）。'

    os=$(uname -s) || fail 'OSを判定できません。'
    cpu=$(uname -m) || fail 'CPUを判定できません。'
    case "$os:$cpu" in
        Darwin:arm64) target=aarch64-apple-darwin ;;
        Linux:x86_64) target=x86_64-unknown-linux-musl ;;
        Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
        *) fail 'このOS・CPUは対応していません。' ;;
    esac

    [ -n "$work" ] || work=$(mktemp -d "${TMPDIR:-/tmp}/miyaburu-install.XXXXXX") || fail '一時領域を作成できません。'
    archive="miyaburu-${target}.tar.gz"
    url="$base/releases/download/$version"
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location "$url/$archive" --output "$work/$archive" || fail 'アーカイブを取得できません。'
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location "$url/$archive.sha256" --output "$work/$archive.sha256" || fail 'チェックサムを取得できません。'

    line=$(awk 'NR == 1 { print; next } { bad=1 } END { if (NR != 1 || bad) exit 1 }' "$work/$archive.sha256") || fail 'チェックサム形式が不正です。'
    expected=${line%%  *}
    printf '%s\n' "$expected" | grep -Eq '^[0-9a-fA-F]{64}$' || fail 'チェックサム形式が不正です。'
    [ "$line" = "$expected  $archive" ] || fail 'チェックサム形式が不正です。'
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$work/$archive") || fail 'SHA-256を計算できません。'
        actual=${actual%% *}
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$work/$archive") || fail 'SHA-256を計算できません。'
        actual=${actual%% *}
    else
        fail 'SHA-256計算ツール（sha256sum または shasum）が必要です。'
    fi
    [ "$expected" = "$actual" ] || fail 'チェックサムが一致しません。'

    mkdir -p "$work/extracted" || fail '一時展開先を作成できません。'
    tar -xzf "$work/$archive" -C "$work/extracted" miyaburu || fail 'アーカイブを展開できません。'
    [ -f "$work/extracted/miyaburu" ] && [ ! -L "$work/extracted/miyaburu" ] || fail 'アーカイブ内の miyaburu が通常ファイルではありません。'

    mkdir -p "$install_dir" || fail '導入先を作成できません。'
    destination="$install_dir/miyaburu"
    if [ -e "$destination" ] || [ -L "$destination" ]; then
        [ ! -L "$destination" ] && [ -f "$destination" ] || fail '既存の miyaburu が通常ファイルではありません。'
    fi
    staged=$(mktemp "$install_dir/.miyaburu.XXXXXX") || fail '導入先に一時ファイルを作成できません。'
    cp "$work/extracted/miyaburu" "$staged" || fail 'バイナリを配置できません。'
    chmod 0755 "$staged" || fail '実行権限を設定できません。'
    mv -f "$staged" "$destination" || fail 'バイナリを置換できません。'
    staged=
    printf 'miyaburu %s を %s にインストールしました。\n' "$version" "$destination"
    case :${PATH-}: in *:"$install_dir":*) ;; *) printf '案内: %s を PATH に追加してください。\n' "$install_dir" ;; esac
    if ! command -v cloudflared >/dev/null 2>&1; then printf '案内: 共有機能を使う場合は cloudflared を別途インストールしてください。\n'; fi
}

main "$@"
