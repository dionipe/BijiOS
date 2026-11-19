#!/bin/bash
set -euo pipefail

REMOTE="${1:-dionipe@192.168.202.4}"
REMOTE_DIR="/home/dionipe/apps/bijios-desktop"
SRC_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "🚀 Mengirim project BijiOS Desktop Environment ke ${REMOTE}:${REMOTE_DIR} (cukup 1x input password SSH)..."

tar -C "${SRC_DIR}" -czf - \
  Cargo.toml \
  Cargo.lock \
  src \
  static \
  packaging \
  deploy-to-bijios.sh \
  dist/bijios-desktop_1.0.0_amd64.deb \
  target/x86_64-unknown-linux-musl/release/bijios-desktop \
  | ssh "${REMOTE}" "mkdir -p '${REMOTE_DIR}' && tar -xzf - -C '${REMOTE_DIR}'"

echo ""
echo "✅ Seluruh project BijiOS Desktop Environment, binary release, dan paket .deb berhasil dibuat di:"
echo "   ${REMOTE}:${REMOTE_DIR}"
echo ""
echo "👉 Untuk langsung menginstal & mengaktifkan tampilan grafis BijiOS Desktop di ${REMOTE}, jalankan:"
echo "   ssh -t ${REMOTE} 'sudo apt update && sudo apt install -y ${REMOTE_DIR}/dist/bijios-desktop_1.0.0_amd64.deb && sudo systemctl restart lightdm'"
