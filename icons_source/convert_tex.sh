#!/bin/bash

# convert_tex.sh
# --------------
# Simple helper script used by the project to turn a LaTeX file into an
# SVG icon.  It runs pdflatex then pdf2svg, placing the resulting files in
# the same directory as the source.
#
# Usage:
#   ./convert_tex.sh path/to/file.tex
#
# If no argument is provided it defaults to "fraction.tex" (the sample file
# that originally lived in the repo).

set -e

# Require a TeX file as the first argument
if [ "$#" -lt 1 ]; then
    echo "❌  No TeX file specified."
    echo "Usage: $0 path/to/file.tex"
    exit 1
fi

TEX_FILE="$1"

if [ ! -f "$TEX_FILE" ]; then
    echo "❌  TeX file not found: $TEX_FILE"
    echo "Usage: $0 path/to/file.tex"
    exit 1
fi

# derive directory and basename so outputs land alongside input
INPUT_DIR=$(dirname "$TEX_FILE")
INPUT_FILE=$(basename "$TEX_FILE")
BASE_NAME="${INPUT_FILE%.tex}"

# ── 1. Check dependencies ────────────────────────────────────────────────────
for cmd in pdflatex pdf2svg; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "❌  '$cmd' not found. Please install it and retry."
        echo ""
        case "$cmd" in
            pdflatex) echo "    Ubuntu/Debian : sudo apt install texlive-latex-base texlive-fonts-recommended" ;;
            pdf2svg)  echo "    Ubuntu/Debian : sudo apt install pdf2svg" ;;
        esac
        exit 1
    fi
done

# ── 2. Compile LaTeX → PDF ───────────────────────────────────────────────────
# run pdflatex in the directory containing the tex file so auxiliary files
# and outputs land next to the source
pushd "$INPUT_DIR" >/dev/null

echo "📐  Compiling $INPUT_FILE → ${BASE_NAME}.pdf …"
pdflatex -interaction=nonstopmode "$INPUT_FILE" > /dev/null 2>&1

if [ ! -f "${BASE_NAME}.pdf" ]; then
    echo "❌  pdflatex failed. Check ${BASE_NAME}.log for details."
    popd >/dev/null
    exit 1
fi

echo "✅  PDF created: ${INPUT_DIR}/${BASE_NAME}.pdf"

popd >/dev/null

# ── 3. Convert PDF → SVG ─────────────────────────────────────────────────────
echo "🔄  Converting ${INPUT_DIR}/${BASE_NAME}.pdf → ${INPUT_DIR}/${BASE_NAME}.svg …"
pdf2svg "${INPUT_DIR}/${BASE_NAME}.pdf" "${INPUT_DIR}/${BASE_NAME}.svg"

if [ ! -f "${INPUT_DIR}/${BASE_NAME}.svg" ]; then
    echo "❌  pdf2svg failed."
    exit 1
fi

echo "✅  SVG created: ${INPUT_DIR}/${BASE_NAME}.svg"

# ── 4. Clean up auxiliary LaTeX files ────────────────────────────────────────
echo "🧹  Cleaning up auxiliary files …"
rm -f "$INPUT_DIR/${BASE_NAME}.aux" \
      "$INPUT_DIR/${BASE_NAME}.log" \
      "$INPUT_DIR/${BASE_NAME}.fls" \
      "$INPUT_DIR/${BASE_NAME}.fdb_latexmk" \
      "$INPUT_DIR/${BASE_NAME}.synctex.gz"

echo ""
echo "🎉  Done! Output: ${INPUT_DIR}/${BASE_NAME}.svg"
