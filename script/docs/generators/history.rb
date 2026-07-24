# docs/history/ — dated writing, frozen on publication.
#
# These summaries have to be *stable*: a frozen file cannot host a line like
# "half of this has shipped", which changes as more ships. What is current
# lives in ROADMAP and the issues, exactly as the folder's contract says.

require_relative "../lib/shared"

generates("history", index: "docs/history/README.md") { summary_index("history") }
