#!/usr/bin/env bash
# Compila 7-Zip (7zz) y par2cmdline-turbo para Android (arm64) con el NDK y los deja como
# bibliotecas nativas del APK: lib7zz.so y libpar2.so. Android solo deja ejecutar programas
# que estén en la carpeta de bibliotecas de la app, por eso llevan ese nombre.
#
# Uso: NDK_HOME=/ruta/al/ndk ./build-native-tools.sh   (lo ejecuta el workflow de GitHub)
set -euo pipefail

SEVENZIP_VERSION=25.01
PAR2_VERSION=1.5.0
API=24

NDK="${NDK_HOME:-${ANDROID_NDK_HOME:-}}"
[ -d "$NDK" ] || { echo "Falta NDK_HOME" >&2; exit 1; }
TC="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
CC="$TC/aarch64-linux-android$API-clang"
CXX="$TC/aarch64-linux-android$API-clang++"
STRIP="$TC/llvm-strip"

HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="$HERE/app/src/main/jniLibs/arm64-v8a"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUT"
JOBS="$(nproc)"

# ---- 7-Zip ----
echo "::group::7-Zip $SEVENZIP_VERSION"
v="${SEVENZIP_VERSION/./}"
curl -fsSL --retry 3 "https://github.com/ip7z/7zip/releases/download/$SEVENZIP_VERSION/7z$v-src.tar.xz" -o "$WORK/7z.tar.xz"
mkdir -p "$WORK/7z"
tar -xJf "$WORK/7z.tar.xz" -C "$WORK/7z"
# El NDK no tiene libpthread aparte (va dentro de libc): una vacía para que enlace.
mkdir -p "$WORK/stub"
"$TC/llvm-ar" rc "$WORK/stub/libpthread.a"
(
  cd "$WORK/7z/CPP/7zip/Bundles/Alone2"
  make -j"$JOBS" -f ../../cmpl_clang_arm64.mak \
    CC="$CC" CXX="$CXX" \
    CFLAGS_WARN_WALL="-Wall" \
    LDFLAGS_STATIC_2="-L$WORK/stub -static-libstdc++"
)
SEVENZ="$(find "$WORK/7z/CPP/7zip/Bundles/Alone2" -type f -name 7zz | head -1)"
[ -n "$SEVENZ" ] || { echo "No se generó 7zz" >&2; exit 1; }
cp "$SEVENZ" "$OUT/lib7zz.so"
echo "::endgroup::"

# ---- par2cmdline-turbo ----
echo "::group::par2cmdline-turbo $PAR2_VERSION"
curl -fsSL --retry 3 "https://github.com/animetosho/par2cmdline-turbo/archive/refs/tags/v$PAR2_VERSION.tar.gz" -o "$WORK/par2.tar.gz"
mkdir -p "$WORK/par2"
tar -xzf "$WORK/par2.tar.gz" -C "$WORK/par2" --strip-components=1
(
  cd "$WORK/par2"
  ./automake.sh
  ./configure --host=aarch64-linux-android CC="$CC" CXX="$CXX" AR="$TC/llvm-ar" RANLIB="$TC/llvm-ranlib" \
    LDFLAGS="-static-libstdc++"
  make -j"$JOBS"
)
cp "$WORK/par2/par2" "$OUT/libpar2.so"
echo "::endgroup::"

"$STRIP" "$OUT/lib7zz.so" "$OUT/libpar2.so"
chmod 755 "$OUT/lib7zz.so" "$OUT/libpar2.so"
file "$OUT"/*.so || true
ls -la "$OUT"
