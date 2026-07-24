# What each generated index section contains, and how to find it in a file.
#
# Shared by script/generate_docs (which writes) and doc_checks/generated.rb
# (which verifies), so the two can never disagree about what "current"
# means.

require_relative "shared"

GENERATED_BEGIN = "<!-- generated: do not edit by hand — script/generate_docs -->"
GENERATED_END = "<!-- /generated -->"

# The italic line under a doc's H1 — the same shape as the `_For: …_`
# audience line, readable in the file itself rather than metadata that only
# an index consumes.
def subtitle(path)
  lines = read_lines(path)
  found = lines[1..4].to_a.find { |line| line.start_with?("_") && line.strip.end_with?("_") }
  return nil unless found

  found.strip.delete_prefix("_").delete_suffix("_")
end

def heading(path)
  found = read_lines(path).first
  return nil unless found&.start_with?("# ")

  found.delete_prefix("# ").strip
end

def ledger_index
  entries = Dir.glob("#{REPO}/docs/ruby/*.md")
               .reject { |path| File.basename(path) == "README.md" }
               .sort

  entries.map do |path|
    name = File.basename(path)
    title = heading(path) or abort "#{relative(path)}: no `# ` heading to index"
    line = subtitle(path) or abort "#{relative(path)}: no italic summary line under the heading"

    "- [#{title}](#{name}) — #{line}"
  end.join("\n")
end

# An ADR's H1 is already an index line — `# 0021 — Namespaces: …` — so unlike
# the ledger files it needs no separate summary. Adding one would be a second
# home for the same sentence.
def adr_index
  Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort.map do |path|
    name = File.basename(path)
    number = name[0, 4]

    title = heading(path) or abort "#{relative(path)}: no `# ` heading to index"
    _, description = title.split(" — ", 2)
    abort "#{relative(path)}: heading is not `NNNN — description`" if description.to_s.empty?

    "- [#{number}](#{name}) — #{description}#{adr_status_note(path)}"
  end.join("\n")
end

# Read the status rather than writing it into the description by hand, so an
# ADR going Tentative → Accepted updates its own index line.
def adr_status_note(path)
  line = read_lines(path).find { |candidate| candidate.start_with?("- **Status:**") }
  abort "#{relative(path)}: no `- **Status:**` line" if line.nil?

  status = line.delete_prefix("- **Status:**").strip

  case status
  when /\ATentative/ then " _(tentative)_"
  when /\ASuperseded by (\d+)/i then " _(superseded by #{Regexp.last_match(1)})_"
  else ""
  end
end

GENERATED_SECTIONS = [
  { index: "docs/ruby/README.md", build: method(:ledger_index) },
  { index: "docs/adr/README.md", build: method(:adr_index) }
].freeze

# The text between the markers, or nil when they are absent.
def generated_section(contents)
  first = contents.index(GENERATED_BEGIN)
  last = contents.index(GENERATED_END)
  return nil if first.nil? || last.nil?

  contents[(first + GENERATED_BEGIN.length)...last].strip
end

def replace_generated_section(contents, replacement)
  first = contents.index(GENERATED_BEGIN)
  last = contents.index(GENERATED_END)
  return nil if first.nil? || last.nil?

  head = contents[0...(first + GENERATED_BEGIN.length)]
  tail = contents[last..]

  "#{head}\n\n#{replacement}\n\n#{tail}"
end

def diff_summary(actual, expected)
  actual_lines = actual.lines.map(&:chomp)
  expected_lines = expected.lines.map(&:chomp)

  added = expected_lines - actual_lines
  removed = actual_lines - expected_lines

  summary = []
  added.each { |line| summary << "  + #{line}" }
  removed.each { |line| summary << "  - #{line}" }
  summary << "  (same lines, different order)" if summary.empty?
  summary.join("\n")
end
