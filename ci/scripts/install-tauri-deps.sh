#!/bin/bash

# Reference: https://v2.tauri.app/start/prerequisites/#system-dependencies
install_on_debian() {
    sudo apt update
    sudo apt install libwebkit2gtk-4.1-dev \
        build-essential \
        curl \
        wget \
        file \
        libxdo-dev \
        libssl-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev
}

install_on_arch() {
    sudo pacman -Syu
    sudo pacman -S --needed \
        webkit2gtk-4.1 \
        base-devel \
        curl \
        wget \
        file \
        openssl \
        appmenu-gtk-module \
        libappindicator-gtk3 \
        librsvg
}

install_on_fedora() {
    sudo dnf check-update
    sudo dnf install webkit2gtk4.1-devel \
        openssl-devel \
        curl \
        wget \
        file \
        libappindicator-gtk3-devel \
        librsvg2-devel
    sudo dnf group install "c-development"
}

install_on_gentoo() {
    sudo emerge --ask \
        net-libs/webkit-gtk:4.1 \
        dev-libs/libappindicator \
        net-misc/curl \
        net-misc/wget \
        sys-apps/file
}

install_on_opensuse() {
    sudo zypper up
    sudo zypper in webkit2gtk3-devel \
        libopenssl-devel \
        curl \
        wget \
        file \
        libappindicator3-1 \
        librsvg-devel
    sudo zypper in -t pattern devel_basis
}

install_on_alpine() {
    sudo apk add \
        build-base \
        webkit2gtk \
        curl \
        wget \
        file \
        openssl \
        libayatana-appindicator-dev \
        librsvg
}

if [ -f /etc/os-release ]; then
    . /etc/os-release
    case $ID in
        debian | ubuntu | linuxmint) install_on_debian;;
        arch | manjaro | endeavouros) install_on_arch ;;
        fedora | rhel | centos) install_on_fedora ;;
        gentoo) install_on_gentoo ;;
        opensuse* | suse) install_on_opensuse ;;
        alpine) install_on_alpine ;;
        *)
            echo "Unsupported distribution: $ID"
            exit 1
            ;;
    esac
elif uname -s | grep -q 'NT'; then
    echo "Running on WSL, skip"
    exit 0
else
    echo "Unsupported Operating System"
    exit 1
fi
