# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

function assert_installed {
    local bin="$1"

    if ! [ -x "$(which "$bin" 2> /dev/null)" ]; then
        echo "error: $bin not installed." >&2
        exit 1
    fi
}

function project_name {
    cargo pkgid | tac -s'/' | head -1 | cut -d'#' -f1
}

function project_version {
    cargo pkgid | tac -s'/' | head -1 | cut -d'#' -f2
}
