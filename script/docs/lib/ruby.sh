# Select a Ruby the way script/test selects a Rust: the script's job, not the
# caller's. Git hooks do not inherit an interactive shell, so without this the
# pre-commit hook runs macOS's system Ruby 2.6, which cannot even parse the
# checks — a failure that reads as a docs error and is not one.
#
# Sourced by script/docs/check and script/docs/generate.

CHRUBY=/opt/homebrew/opt/chruby/share/chruby/chruby.sh
if [ -f "$CHRUBY" ]; then
  . "$CHRUBY"
  chruby "$(cat "$HOME/.ruby-version" 2> /dev/null)" 2> /dev/null || chruby ruby 2> /dev/null || true
fi

if ! ruby -e 'exit RUBY_VERSION.split(".").first.to_i >= 3' 2> /dev/null; then
  echo "needs Ruby 3+, found $(ruby -v 2>&1 | cut -d' ' -f2)" >&2
  echo "  install one with chruby, or put a modern ruby on PATH" >&2
  exit 1
fi
