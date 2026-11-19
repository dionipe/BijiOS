#!/bin/bash
# BijiOS Desktop — Storytold Open-Source Office Suite Bundler (Apache-2.0)
# Downloads and installs WordCraft, GridCraft, DeckCraft, and PDFCraft (.deb)
# Credits: Storytold / ArtCraft (https://github.com/storytold) — Apache License 2.0

set -euo pipefail

TMP_DIR="/tmp/bijios-storytold-bundle"
mkdir -p "${TMP_DIR}"

WORDCRAFT_VER="0.18.1"
GRIDCRAFT_VER="0.19.1"
DECKCRAFT_VER="0.18.1"
PDFCRAFT_VER="0.21.1"

echo "📦 Downloading Storytold Pure Rust Office Suite (Apache-2.0)..."
curl -fsSL "https://github.com/storytold/wordcraft/releases/download/v${WORDCRAFT_VER}/wordcraft-${WORDCRAFT_VER}-linux-x86_64.deb" -o "${TMP_DIR}/wordcraft.deb"
curl -fsSL "https://github.com/storytold/gridcraft/releases/download/v${GRIDCRAFT_VER}/gridcraft-${GRIDCRAFT_VER}-linux-x86_64.deb" -o "${TMP_DIR}/gridcraft.deb"
curl -fsSL "https://github.com/storytold/deckcraft/releases/download/v${DECKCRAFT_VER}/deckcraft-${DECKCRAFT_VER}-linux-x86_64.deb" -o "${TMP_DIR}/deckcraft.deb"
curl -fsSL "https://github.com/storytold/pdfcraft/releases/download/v${PDFCRAFT_VER}/pdfcraft-${PDFCRAFT_VER}-linux-x86_64.deb" -o "${TMP_DIR}/pdfcraft.deb"

echo "⚙️  Installing WordCraft, GridCraft, DeckCraft, and PDFCraft..."
sudo apt-get install -y \
  "${TMP_DIR}/wordcraft.deb" \
  "${TMP_DIR}/gridcraft.deb" \
  "${TMP_DIR}/deckcraft.deb" \
  "${TMP_DIR}/pdfcraft.deb"

rm -rf "${TMP_DIR}"
echo "✅ Storytold Office Suite (WordCraft, GridCraft, DeckCraft, PDFCraft — Apache-2.0) installed!"
