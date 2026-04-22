#!/bin/bash
# Debian package builder for morph
# Adapted from sharkdp/fd's create-deb.sh
set -euo pipefail

COPYRIGHT_YEARS="2025"
MAINTAINER="alvinreal <alvinreal@users.noreply.github.com>"
REPO="https://github.com/alvinreal/morph"
DPKG_STAGING="${CICD_INTERMEDIATES_DIR:-.}/debian-package"
DPKG_DIR="${DPKG_STAGING}/dpkg"
mkdir -p "${DPKG_DIR}"

if [[ -z "$TARGET" ]]; then
  TARGET="$(rustc -vV | sed -n 's|host: \(.*\)|\1|p')"
fi

case "$TARGET" in
  *-musl*)
    DPKG_BASENAME=morph-musl
    DPKG_CONFLICTS="morph"
    ;;
  *)
    DPKG_BASENAME=morph
    DPKG_CONFLICTS="morph-musl"
    ;;
esac

if [[ -z "$DPKG_VERSION" ]]; then
  DPKG_VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r .packages[0].version)
fi

unset DPKG_ARCH
case "${TARGET}" in
  aarch64-*-linux-*) DPKG_ARCH=arm64 ;;
  arm-*-linux-*hf) DPKG_ARCH=armhf ;;
  i686-*-linux-*) DPKG_ARCH=i386 ;;
  x86_64-*-linux-*) DPKG_ARCH=amd64 ;;
  *) DPKG_ARCH=notset ;;
esac;

DPKG_NAME="${DPKG_BASENAME}_${DPKG_VERSION}_${DPKG_ARCH}.deb"

BIN_PATH=${BIN_PATH:-target/${TARGET}/release/morph}

# Binary
install -Dm755 "${BIN_PATH}" "${DPKG_DIR}/usr/bin/morph"

# Man page (if exists)
if [[ -f "doc/morph.1" ]]; then
  install -Dm644 'doc/morph.1' "${DPKG_DIR}/usr/share/man/man1/morph.1"
  gzip -n --best "${DPKG_DIR}/usr/share/man/man1/morph.1"
fi

# Autocompletion files (if they exist)
if [[ -d "autocomplete" ]]; then
  [[ -f "autocomplete/morph.bash" ]] && install -Dm644 'autocomplete/morph.bash' "${DPKG_DIR}/usr/share/bash-completion/completions/morph"
  [[ -f "autocomplete/morph.fish" ]] && install -Dm644 'autocomplete/morph.fish' "${DPKG_DIR}/usr/share/fish/vendor_completions.d/morph.fish"
  [[ -f "autocomplete/_morph" ]] && install -Dm644 'autocomplete/_morph' "${DPKG_DIR}/usr/share/zsh/vendor-completions/_morph"
fi

# README and LICENSE
install -Dm644 "README.md" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/README.md"
[[ -f "LICENSE" ]] && install -Dm644 "LICENSE" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/LICENSE"
[[ -f "LICENSE-MIT" ]] && install -Dm644 "LICENSE-MIT" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/LICENSE-MIT"
[[ -f "LICENSE-APACHE" ]] && install -Dm644 "LICENSE-APACHE" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/LICENSE-APACHE"
[[ -f "CHANGELOG.md" ]] && install -Dm644 "CHANGELOG.md" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/changelog"
[[ -f "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/changelog" ]] && gzip -n --best "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/changelog"

# Copyright file
cat > "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/copyright" <<EOF
Format: http://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: morph
Source: ${REPO}

Files: *
Copyright: ${COPYRIGHT_YEARS} ${MAINTAINER}
License: MIT

License: MIT
  Permission is hereby granted, free of charge, to any
  person obtaining a copy of this software and associated
  documentation files (the "Software"), to deal in the
  Software without restriction, including without
  limitation the rights to use, copy, modify, merge,
  publish, distribute, sublicense, and/or sell copies of
  the Software, and to permit persons to whom the Software
  is furnished to do so, subject to the following
  conditions:
  .
  The above copyright notice and this permission notice
  shall be included in all copies or substantial portions
  of the Software.
  .
  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
  ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
  TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
  PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
  SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
  CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
  OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
  IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
  DEALINGS IN THE SOFTWARE.
EOF
chmod 644 "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/copyright"

# control file
mkdir -p "${DPKG_DIR}/DEBIAN"
cat > "${DPKG_DIR}/DEBIAN/control" <<EOF
Package: ${DPKG_BASENAME}
Version: ${DPKG_VERSION}
Section: utils
Priority: optional
Maintainer: ${MAINTAINER}
Homepage: ${REPO}
Architecture: ${DPKG_ARCH}
Provides: morph
Conflicts: ${DPKG_CONFLICTS}
Description: Universal data format converter with mapping language
  morph is a universal CLI data format converter with a mapping language.
  It supports JSON, YAML, TOML, CSV, XML, MessagePack, and JSON Lines formats.
  Features include a mapping language with rename, select, drop, set, cast,
  where, each, when, flatten, nest, sort, default operations, and 30+ built-in
  functions for string, math, collection, and type operations.
EOF

DPKG_PATH="${DPKG_STAGING}/${DPKG_NAME}"

if [[ -n $GITHUB_OUTPUT ]]; then
  echo "DPKG_NAME=${DPKG_NAME}" >> "$GITHUB_OUTPUT"
  echo "DPKG_PATH=${DPKG_PATH}" >> "$GITHUB_OUTPUT"
fi

# build dpkg
fakeroot dpkg-deb --build "${DPKG_DIR}" "${DPKG_PATH}"
