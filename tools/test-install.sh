#!/bin/sh
# install.sh の配布・検証経路をネットワークなしで回帰確認します。
set -eu
root=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/miyaburu-test-install.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
fail() { printf '失敗: %s\n' "$*" >&2; exit 1; }
ok() { printf '確認: %s\n' "$*"; }
sh -n "$root/install.sh" || fail 'install.sh の構文検査'
mkdir -p "$work/bin" "$work/fixture"
# Stub scripts are written literally so their variables expand only when executed.
# shellcheck disable=SC2016
cat > "$work/bin/uname" <<'STUB'
#!/bin/sh
case $1 in -s) printf '%s\n' "$TEST_OS" ;; -m) printf '%s\n' "$TEST_CPU" ;; *) exit 1 ;; esac
STUB
# shellcheck disable=SC2016
cat > "$work/bin/curl" <<'STUB'
#!/bin/sh
set -eu
url= out=
while [ "$#" -gt 0 ]; do case $1 in --output) out=$2; shift 2;; --write-out) shift 2;; https://*) url=$1; shift;; *) shift;; esac; done
case $url in
    */releases/latest) [ "${TEST_FAIL_LATEST-}" != 1 ] || exit 22; printf '%s' 'https://github.com/salan70/miyaburu/releases/tag/v1.2.3' ;;
    */releases/download/*)
        [ "${TEST_FAIL_DOWNLOAD-}" != 1 ] || exit 22
        rest=${url#*/releases/download/}; version=${rest%%/*}
        [ "$version" = "${TEST_EXPECT_VERSION-v1.2.3}" ] || exit 22
        name=${url##*/}; [ -f "$TEST_FIXTURE/$name" ] || exit 22; cp "$TEST_FIXTURE/$name" "$out" ;;
    *) exit 22 ;;
esac
STUB
chmod +x "$work/bin/uname" "$work/bin/curl"
printf '#!/bin/sh\necho miyaburu\n' > "$work/payload"
for target in aarch64-apple-darwin x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
    mkdir -p "$work/pack/$target"
    cp "$work/payload" "$work/pack/$target/miyaburu"
    tar -czf "$work/fixture/miyaburu-$target.tar.gz" -C "$work/pack/$target" miyaburu
    (cd "$work/fixture" && if command -v sha256sum >/dev/null 2>&1; then sha256sum "miyaburu-$target.tar.gz"; else shasum -a 256 "miyaburu-$target.tar.gz"; fi | awk '{print $1 "  " $2}' > "miyaburu-$target.tar.gz.sha256")
done
invoke() {
    os=$1 cpu=$2 dest=$3; shift 3
    env PATH="$work/bin:$PATH" TEST_OS="$os" TEST_CPU="$cpu" TEST_FIXTURE="$work/fixture" MIYABURU_INSTALL_DIR="$dest" "$@" sh "$root/install.sh"
}
expect_success() { invoke "$@" >/dev/null 2>&1 || fail "導入成功を期待: $1/$2"; }
expect_failure() { if invoke "$@" >/dev/null 2>&1; then fail "失敗を期待: $1/$2"; fi; }
while read -r os cpu target; do
    dest="$work/targets/$target"; expect_success "$os" "$cpu" "$dest"; [ -f "$dest/miyaburu" ] || fail "$target 配置"; ok "$target"
done <<'TARGETS'
Darwin arm64 aarch64-apple-darwin
Linux x86_64 x86_64-unknown-linux-musl
Linux aarch64 aarch64-unknown-linux-musl
TARGETS
# 最新版固定、および v あり・なしの明示指定。導入先に空白を含めます。
dest="$work/path with spaces"
expect_success Linux x86_64 "$dest" env
MIYABURU_VERSION=1.2.4 TEST_EXPECT_VERSION=v1.2.4; export MIYABURU_VERSION TEST_EXPECT_VERSION
expect_success Linux x86_64 "$dest" env
MIYABURU_VERSION=v1.2.4; export MIYABURU_VERSION
expect_success Linux x86_64 "$dest" env
unset MIYABURU_VERSION TEST_EXPECT_VERSION
ok '最新版・版指定・空白を含む導入先'
# 案内は成功時のみ出力すること。
output=$(env PATH="$work/bin:/usr/bin:/bin" TEST_OS=Linux TEST_CPU=x86_64 TEST_FIXTURE="$work/fixture" MIYABURU_INSTALL_DIR="$work/advice" sh "$root/install.sh" 2>&1) || fail '案内確認用導入'
case $output in *'PATH に追加'*'cloudflared を別途'*) ;; *) fail 'PATH/cloudflared 案内' ;; esac
# 破損チェックサム・欠落・ダウンロード失敗は既存ファイルを保持。
dest="$work/keep"; mkdir -p "$dest"; printf old > "$dest/miyaburu"
cp -R "$work/fixture" "$work/badfixture"
printf '%064d  miyaburu-x86_64-unknown-linux-musl.tar.gz\n' 0 > "$work/badfixture/miyaburu-x86_64-unknown-linux-musl.tar.gz.sha256"
if env PATH="$work/bin:$PATH" TEST_OS=Linux TEST_CPU=x86_64 TEST_FIXTURE="$work/badfixture" MIYABURU_INSTALL_DIR="$dest" sh "$root/install.sh" >/dev/null 2>&1; then fail 'checksum不一致' ; fi
[ "$(cat "$dest/miyaburu")" = old ] || fail 'checksum不一致で既存ファイルを保持'
rm "$work/badfixture/miyaburu-x86_64-unknown-linux-musl.tar.gz.sha256"
if env PATH="$work/bin:$PATH" TEST_OS=Linux TEST_CPU=x86_64 TEST_FIXTURE="$work/badfixture" MIYABURU_INSTALL_DIR="$dest" sh "$root/install.sh" >/dev/null 2>&1; then fail 'checksum欠落' ; fi
if env PATH="$work/bin:$PATH" TEST_OS=Linux TEST_CPU=x86_64 TEST_FIXTURE="$work/fixture" TEST_FAIL_DOWNLOAD=1 MIYABURU_INSTALL_DIR="$dest" sh "$root/install.sh" >/dev/null 2>&1; then fail 'ダウンロード失敗' ; fi
[ "$(cat "$dest/miyaburu")" = old ] || fail '失敗時に既存ファイルを保持'
ok 'checksum・ダウンロード失敗と既存ファイル保持'
# 不正版、未対応環境、symlink拒否。
if env PATH="$work/bin:$PATH" TEST_OS=Linux TEST_CPU=x86_64 TEST_FIXTURE="$work/fixture" MIYABURU_INSTALL_DIR="$work/invalid" MIYABURU_VERSION=1.2.3-rc1 sh "$root/install.sh" >/dev/null 2>&1; then fail '不正version' ; fi
expect_failure FreeBSD x86_64 "$work/unsupported" env
expect_failure Linux i386 "$work/unsupported" env
mkdir -p "$work/linkdest"; ln -s "$work/keep/miyaburu" "$work/linkdest/miyaburu"
expect_failure Linux x86_64 "$work/linkdest" env
[ -L "$work/linkdest/miyaburu" ] || fail 'symlinkを保持'
ok '不正version・非対応環境・symlink拒否'
printf 'すべてのインストーラーテストに合格しました。\n'
