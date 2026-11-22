"""T1624: deterministic source download for isolated package regressions."""


def install(root):
    # T1624: package orchestration uses a bounded HTTPS source download.
    curl = root / 'bin/curl'
    curl.parent.mkdir(parents=True, exist_ok=True)
    curl.write_text("""#!/bin/bash
set -eu
[ "$1" = --fail ] && [ "$2" = --location ]
[ "$3" = --proto ] && [ "$4" = =https ]
[ "$5" = --proto-redir ] && [ "$6" = =https ]
[ "$7" = --max-filesize ] && [ "$8" = 134217728 ]
[ "$9" = --output ]
[[ "${11}" = https://github.com/geraldo-netto/blent/archive/refs/tags/v*.tar.gz ]]
[ "$(ulimit -f)" = 131072 ]
printf 'partial download' > "${10}"
[ "${BLENT_TEST_MODE:-}" != archive-failure ] || exit 22
printf 'T1624 release archive bytes' > "${10}"
""")
    curl.chmod(0o755)
