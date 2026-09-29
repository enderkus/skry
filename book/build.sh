#!/bin/sh
# Builds the GitHub Pages site: the landing page from site/ and both
# documentation books from book/{en,tr}.
#
#   sh book/build.sh [OUTPUT_DIR]          (needs mdbook on PATH, or MDBOOK=/path/to/mdbook)
set -eu
cd "$(dirname "$0")/.."
out=${1:-_site}
mdbook=${MDBOOK:-mdbook}
rm -rf "$out"
mkdir -p "$out/assets"
cp -R site/. "$out/"
cp docs/demo.gif "$out/assets/demo.gif"
root=$(pwd)
case "$out" in /*) dest=$out ;; *) dest=$root/$out ;; esac
"$mdbook" build book/en -d "$dest/docs/en"
"$mdbook" build book/tr -d "$dest/docs/tr"
touch "$out/.nojekyll"
echo "site built in $out"
