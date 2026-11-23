# The macOS this repo builds for (#444). Sourced, not run.
#
# `.macos-version` at the repo root names the minimum macOS a Portland build
# supports, which is the newest one: there is nothing older to support
# before a first public release. A developer's machine must be on it. CI's
# runner image can trail the newest point release, so there the check
# reports and a build uses the runner's own version.

# Read from the repo root, where every script that sources this has gone.
macos_wanted=$(cat .macos-version)
macos_installed=$(sw_vers -productVersion)

# The older of two versions, by their numbers.
macos_older_of() {
  printf '%s\n%s\n' "$1" "$2" | sort -V | head -1
}

# Says where this machine stands against the file, and refuses a
# developer's machine that is behind it.
macos_check() {
  if [ "$macos_installed" = "$macos_wanted" ]; then
    echo "macOS: $macos_installed"
  elif [ "$(macos_older_of "$macos_installed" "$macos_wanted")" = "$macos_wanted" ]; then
    echo "macOS: $macos_installed, ahead of .macos-version ($macos_wanted) — bring the file up"
  elif [ -n "$CI" ]; then
    echo "macOS: $macos_installed, behind .macos-version ($macos_wanted) — the runner trails, so builds here use $macos_installed"
  else
    echo "script/bootstrap: this machine is on macOS $macos_installed, and .macos-version asks for $macos_wanted — update macOS" >&2
    return 1
  fi
}

# The deployment target a build passes to the compiler: the file's version,
# or the machine's own where the machine is behind it (CI).
macos_deployment_target() {
  macos_older_of "$macos_installed" "$macos_wanted"
}
