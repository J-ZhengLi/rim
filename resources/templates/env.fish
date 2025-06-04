# rim shell setup (inspired by rustup)
# DO NOT modify
function add_to_path
    set path_to_add $argv[1]

    if not contains "$name" $PATH
        set -x PATH "$name" $PATH
    end
end
