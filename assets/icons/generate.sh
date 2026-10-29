#!/usr/bin/env bash
set -euo pipefail
SOURCE="assets/icons/combinator.svg"
OUTPUT_DIR="assets/icons"
mkdir -p "$OUTPUT_DIR"
for SIZE in 16 32 48 64 128 256 512; do
  rsvg-convert -w "$SIZE" -h "$SIZE" "$SOURCE" -o "$OUTPUT_DIR/combinator-${SIZE}.png"
done
magick "$OUTPUT_DIR/combinator-256.png" \
  \( "$OUTPUT_DIR/combinator-48.png" \) \
  \( "$OUTPUT_DIR/combinator-32.png" \) \
  \( "$OUTPUT_DIR/combinator-16.png" \) \
  "$OUTPUT_DIR/combinator.ico"