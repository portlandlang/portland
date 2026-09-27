# Put rustup's cargo and rustc first on PATH, from wherever Homebrew keeps
# them — rustup is keg-only, so they are not on a plain PATH until linked, and
# git hooks do not inherit an interactive shell's additions. Homebrew is asked
# rather than assumed: `brew --prefix rustup` answers in hundredths of a
# second. Where there is no Homebrew, or no rustup under it, PATH is left
# alone, and a cargo already on it is used.
#
# Which Rust runs is rust-toolchain.toml's to say; this only finds rustup.
#
# Sourced by script/bootstrap, script/test, script/spec, script/console, and
# the pre-commit hook.

if command -v brew > /dev/null 2>&1; then
  rustup_prefix=$(brew --prefix rustup 2> /dev/null)
  if [ -d "$rustup_prefix/bin" ]; then
    PATH="$rustup_prefix/bin:$PATH"
    export PATH
  fi
fi
