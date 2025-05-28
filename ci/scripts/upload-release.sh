#!/usr/bin/env bash
# Uploads files to XuanWu OBS server

set -euo pipefail

ENDPOINT='https://obs.cn-north-4.myhuaweicloud.com'

CACHE_DIR="`dirname $0`/cache"
echo "cache will be stored at: $CACHE_DIR"
mkdir -p "$CACHE_DIR"

OS=`uname -s`
ARCH=`uname -m`

extract_file() {
    local archive=$1
    local output_dir=$2
    local strip_components=${3:-0}

    case "$archive" in
        *.tar.xz|*.txz|*.tar.gz|*.tgz|*.tar)
            [[ $strip_components -gt 0 ]] && stript_opt=" --strip-components=$strip_components" || stript_opt=""
            tar -xf "$archive" -C "$output_dir"$stript_opt
            ;;
        *.zip)
            local temp="$CACHE_DIR/unzip_temp"
            mkdir -p "$temp"
            rm -rf "$temp"/*

            unzip -q "$archive" -d "$temp"

            if [ "$(ls -1 "$temp" | wc -l)" -eq $strip_components ]; then
                mv -f "$temp"/*/* "$output_dir"
            else
                mv -f "$temp"/* "$output_dir"
            fi
            rm -rf "$temp"
            ;;
        *)
            echo "error: unsupported archive format '$archive'" >&2
            return 3
            ;;
    esac

    echo "successfully extracted '$archive' to '$output_dir'"
}

download_and_install_obsutil() {
    local obsutil_url=''

    case "$OS-$ARCH" in
        "Linux-x86_64")
            obsutil_url='https://obs-community.obs.cn-north-1.myhuaweicloud.com/obsutil/current/obsutil_linux_amd64.tar.gz'
            ;;
        "Linux-aarch64")
            obsutil_url='https://obs-community.obs.cn-north-1.myhuaweicloud.com/obsutil/current/obsutil_linux_arm64.tar.gz'
            ;;
        "Darwin-x86_64")
            obsutil_url='https://obs-community.obs.cn-north-1.myhuaweicloud.com/obsutil/current/obsutil_darwin_amd64.tar.gz'
            ;;
        *)
            if [[ "$OS" == *"NT"* && "$ARCH" == "x86_64" ]]; then
                obsutil_url='https://obs-community.obs.cn-north-1.myhuaweicloud.com/obsutil/current/obsutil_windows_amd64.zip'
            else
                echo "Unsupported operating system or architecture: $OS-$ARCH" >&2
                exit 2
            fi
            ;;
    esac

    # download obsutil archive
    local obsutil_archive_name=`basename $obsutil_url`
    local obsutil_archive="$CACHE_DIR/$obsutil_archive_name"
    echo "downloading obsutil archive from remote server"
    [[ -f "$obsutil_archive" ]] || curl -Lso $obsutil_archive $obsutil_url

    # extract obsutil archive
    local obsutil_path="$CACHE_DIR/obsutil"
    mkdir -p $obsutil_path
    extract_file $obsutil_archive $obsutil_path 1

    # add obsutil to PATH
    export PATH="$obsutil_path:$PATH"
}

retry() {
    local nth_try=1
    local limit=$1
    shift

    while true; do
        "$@" && break || {
            if [[ $nth_try -lt $limit ]]; then
                echo "command failed, retry in $nth_try second(s) ($nth_try/$limit)"
                ((nth_try++))
                sleep $nth_try
            else
                echo "error: command failed after $nth_try attempts, exiting..." >&2
                return 4
            fi
        }
    done
}

download_and_install_obsutil

# config obsutil
obsutil config -i="$ACCESS_KEY_ID" -k="$SECURITY_KEY_ID" -e="$ENDPOINT"
retry 10 obsutil cp -r -f -u -flat "$@"
