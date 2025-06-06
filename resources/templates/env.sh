#!/bin/sh
# rim shell setup (inspired by rustup)
# DO NOT modify
add_to_path() {
    local path_to_add=$1

    case ":${PATH}:" in
        *:"${path_to_add}":*)
            ;;
        *)
            export PATH="${path_to_add}:$PATH"
            ;;
    esac
}
