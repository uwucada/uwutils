#!/bin/bash
#
# build / install script for uwutils

set -e

# output colours

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${GREEN}「info」${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}「warning」${NC} $1"
}

print_error() {
    echo -e "${RED}「error」${NC} $1"
}


# check os
OS="unknown"
if [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
else
    print_error "Unsupported OS: $OSTYPE"
    exit 1
fi

print_status "Detected OS: $OS"

# determine install dir
INSTALL_DIR=""
if [ -d "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
elif [ -d "/usr/local/bin" ]; then

    INSTALL_DIR="/usr/local/bin"
elif [ -d "/usr/bin" ]; then
    INSTALL_DIR="/usr/bin"
else
    mkdir -p "$HOME/.local/bin"
    INSTALL_DIR="$HOME/.local/bin"
    print_warning "Created $HOME/.local/bin directory"
fi
print_status "Installation directory: $INSTALL_DIR"

# check if installdir is on path
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    print_warning "$INSTALL_DIR is not in your PATH"
    print_status "Add the following line to your shell profile:"
    if [ "$OS" = "macos" ]; then
        echo "export PATH=\"\$PATH:$INSTALL_DIR\""
    else
        echo "export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
fi

# build project
print_status "Building Rust workspace in release mode..."
cargo build --release --workspace

if [ $? -ne 0 ]; then
    print_error "Build failed"
    exit 1
fi

print_status "Build successful"

# get target dir to fetch built bins
TARGET_DIR="target/release"

BINARIES=()
for file in "$TARGET_DIR"/*; do
    if [ -f "$file" ] && [ -x "$file" ]; then

        if [ "$OS" = "macos" ] && [[ "$file" == *.dSYM ]]; then
            continue
        fi

        BINARIES+=("$(basename "$file")")

    fi

done

if [ ${#BINARIES[@]} -eq 0 ]; then
    print_error "no binaries found in $TARGET_DIR"
    exit 1
fi

print_status "f binaries: ${BINARIES[*]}"

# install bins
for binary in "${BINARIES[@]}"; do

    source_path="$TARGET_DIR/$binary"
    dest_path="$INSTALL_DIR/$binary"
    print_status "installing $binary to $INSTALL_DIR"

    if [ -f "$dest_path" ]; then
        print_warning "$binary already exists in $INSTALL_DIR"
        read -p "do you want to overwrite it? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_status "skipping $binary"
            continue
        fi
    fi

    # copy the binary
    cp "$source_path" "$dest_path"
    chmod +x "$dest_path"
    if [ $? -eq 0 ]; then
        print_status "$binary installed successfully"
    else
        print_error "failed to install $binary"
    fi
done

print_status "installation complete!"
