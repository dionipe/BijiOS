#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_NAME="bijios-desktop"
PKG_VER="1.0.0"
ARCH="amd64"
BUILD_ROOT="${ROOT_DIR}/packaging/.build/${PKG_NAME}_${PKG_VER}_${ARCH}"
DIST_DIR="${ROOT_DIR}/dist"
BIN_PATH="${ROOT_DIR}/target/release/bijios-desktop"

if [ ! -f "${BIN_PATH}" ]; then
  echo "⚙️  Membangun release binary native Rust + GTK4 bijios-desktop dengan cargo..."
  (cd "${ROOT_DIR}" && cargo build --release)
fi

rm -rf "${BUILD_ROOT}"
mkdir -p "${BUILD_ROOT}" "${DIST_DIR}"

cp -a "${ROOT_DIR}/packaging/deb/." "${BUILD_ROOT}/"

mkdir -p "${BUILD_ROOT}/usr/bin"
cp "${BIN_PATH}" "${BUILD_ROOT}/usr/bin/bijios-desktop"
chmod 755 "${BUILD_ROOT}/usr/bin/bijios-desktop" \
          "${BUILD_ROOT}/usr/bin/bijios-session" \
          "${BUILD_ROOT}/usr/bin/bijios-app-launch" \
          "${BUILD_ROOT}/usr/bin/bijios-app-sync"

mkdir -p "${BUILD_ROOT}/usr/share/bijios-desktop/static"
cp -a "${ROOT_DIR}/static/." "${BUILD_ROOT}/usr/share/bijios-desktop/static/"

chmod 755 "${BUILD_ROOT}/DEBIAN" "${BUILD_ROOT}/DEBIAN/postinst" "${BUILD_ROOT}/DEBIAN/prerm"
chmod 644 "${BUILD_ROOT}/DEBIAN/control"

OUT_DEB="${DIST_DIR}/${PKG_NAME}_${PKG_VER}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "${BUILD_ROOT}" "${OUT_DEB}"

echo ""
echo "🎉 BijiOS v${PKG_VER} (Padi) Full Native Desktop Debian package built:"
echo "   ${OUT_DEB}"
ls -lh "${OUT_DEB}"
