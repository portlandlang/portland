# docs/ruby/ — the Ruby → Portland difference ledger.
#
# Each ledger file's H1 is a bare noun ("Ranges"), so the index needs the
# italic summary line under it to say anything useful.

require_relative "../lib/shared"

generates("ledger", index: "docs/ruby/README.md") { summary_index("ruby") }
