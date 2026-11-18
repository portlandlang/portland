# docs/adr/NNNN-*.md — each ADR's banner (ADR 0089).
#
# An ADR is a decision record: a later ruling can override part of one
# without marking it. The banner sends a reader to the reference docs that
# cite the ADR, built from those citations, so it can neither drift nor be
# forgotten. An ADR no reference doc cites has no banner to build, and the
# generator refuses — a decision recorded only in the log.

require_relative "../lib/shared"

# The reference docs, each fact in one of them. The long ones are cited by
# section; a ledger page is one topic, so it is cited whole.
SECTIONED_REFERENCES = %w[docs/principles.md docs/language.md docs/architecture.md AGENT.md].freeze

def ledger_pages
  Dir.glob("#{REPO}/docs/ruby/*.md")
     .reject { |path| File.basename(path) == "README.md" }
     .sort
     .map { |path| relative(path) }
end

# GitHub's heading anchor: the rendered text, lowercased, with everything
# but letters, digits, `_`, `-`, and spaces dropped, and spaces made hyphens.
def anchor(heading_text)
  rendered = heading_text.gsub(/\[([^\]]*)\]\([^)]*\)/, '\1')
  rendered.downcase.gsub(/[^\p{Word}\- ]/, "").tr(" ", "-")
end

def cites?(line, number)
  line.match?(%r{adr/#{number}-}) || line.match?(/\bADR #{number}\b/)
end

# Every place a reference doc cites the ADR, as [label, target] pairs,
# in reading order and without repeats.
def reference_citations(number)
  sectioned = SECTIONED_REFERENCES.flat_map do |doc|
    section = nil
    read_lines("#{REPO}/#{doc}").filter_map do |line|
      section = line.sub(/\A#+\s*/, "").strip if line.match?(/\A##+ /)
      next unless cites?(line, number)

      label = section ? "#{File.basename(doc)}: #{section.delete("`")}" : File.basename(doc)
      target = section ? "#{doc}##{anchor(section)}" : doc
      [label, target]
    end
  end

  ledger = ledger_pages.filter_map do |doc|
    next unless read_lines("#{REPO}/#{doc}").any? { cites?(it, number) }

    ["docs/ruby/#{File.basename(doc)}", doc]
  end

  (sectioned + ledger).uniq
end

def adr_banner(path)
  number = File.basename(path)[0, 4]
  citations = reference_citations(number)
  if citations.empty?
    abort "#{relative(path)}: no reference doc cites ADR #{number} — add it where its answer lives " \
          "(docs/language.md, docs/architecture.md, docs/principles.md, a docs/ruby/ page, or AGENT.md) (ADR 0089)"
  end

  links = citations.map { |label, target| "[#{label}](../../#{target})" }.join(" · ")
  # A GitHub alert, so the banner renders as a colored callout there.
  "> [!IMPORTANT]\n> **A decision record, not a reference.** It says why a ruling was made; a later ruling may have changed it without marking this file. See also: #{links}."
end

Dir.glob("#{REPO}/docs/adr/[0-9]*.md").sort.each do |path|
  generates("adr_banners", index: relative(path), insert_under_heading: true) { adr_banner(path) }
end
