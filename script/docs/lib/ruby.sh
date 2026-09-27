# Select a Ruby the way script/test selects a Rust: the script's job, not the
# caller's. Git hooks do not inherit an interactive shell, so without this the
# pre-commit hook runs macOS's system Ruby 2.6, which cannot even parse the
# checks — a failure that reads as a docs error and is not one.
#
# rv answers with the Ruby this project pins in .ruby-version, and its
# directory goes first on PATH, so `ruby` and `bundle` both come from it.
# Where there is no rv — CI, which installs its Ruby with ruby/setup-ruby —
# whatever Ruby is on PATH is used, and the version check below holds either
# way.
#
# Sourced by script/docs/check, script/docs/generate, script/bench, and
# script/bootstrap, each after it has moved to the repo root, so rv finds the
# project's pin rather than a home directory's.

if command -v rv > /dev/null 2>&1; then
  pinned_ruby=$(rv ruby find 2> /dev/null) && PATH="$(dirname "$pinned_ruby"):$PATH"
  export PATH
fi

if ! ruby -e 'exit RUBY_VERSION.split(".").first.to_i >= 4' 2> /dev/null; then
  echo "needs Ruby 4+, found $(ruby -v 2>&1 | cut -d' ' -f2)" >&2
  echo "  install the pinned one with \`rv ruby install\`, or put a modern ruby on PATH" >&2
  exit 1
fi
