# docs/adr/ — the decision log.
#
# An ADR's H1 is already an index line — `# 0021 — Namespaces: …` — so unlike
# the ledger files it needs no separate summary. Adding one would be a second
# home for the same sentence.

require_relative "../lib/shared"

# Read the status rather than writing it into the description by hand, so an
# ADR going Tentative → Accepted updates its own index line.
def adr_status_note(path)
  line = read_lines(path).find { |candidate| candidate.start_with?("- **Status:**") }
  abort "#{relative(path)}: no `- **Status:**` line" if line.nil?

  case line.delete_prefix("- **Status:**").strip
  when /\ATentative/ then " _(tentative)_"
  when /\ASuperseded by (\d+)/i then " _(superseded by #{Regexp.last_match(1)})_"
  else ""
  end
end

def adr_index
  Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort.map do |path|
    name = File.basename(path)

    title = heading(path) or abort "#{relative(path)}: no `# ` heading to index"
    _, description = title.split(" — ", 2)
    abort "#{relative(path)}: heading is not `NNNN — description`" if description.to_s.empty?

    "- [#{name[0, 4]}](#{name}) — #{description}#{adr_status_note(path)}"
  end.join("\n")
end

generates("adr", index: "docs/adr/README.md") { adr_index }
