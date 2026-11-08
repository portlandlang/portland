# Everything Portland development needs from Homebrew.
# Install with: brew bundle

brew 'actionlint' # lints .github/workflows
brew 'gh'         # GitHub CLI for repo and issue work
brew 'rustup'     # Rust toolchain for the seed, at rust-toolchain.toml's pin
# Ruby for the doc tooling and scripts, at .ruby-version's pin. CI brings
# its own Ruby (ruby/setup-ruby, from the same pin), so rv is local only.
brew 'rv' unless ENV['CI']
