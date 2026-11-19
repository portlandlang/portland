# No time-relative words in prose (#288; AGENT.md, "Writing that stays
# true").
#
# "today", "not yet", "for now" are true when written and age without a
# sound: ADRs 0002, 0004, and 0011 said `together` was unbuilt for four
# months after it was built. A pending state should name the issue that ends
# it, so that closed_issue_claims can tell when it goes stale.
#
# So a time word passes when its clause cites an issue ("refuses; #291
# tracks it"), and fails otherwise. Docs read the full list; code comments
# read only the words that cannot describe a running program, since "the
# current token" and "names declared so far" are about execution, not the
# calendar. Quoted and backticked text is someone's output or code; "yet" as
# a conjunction (", yet they cost") and compounds like "now-unnecessary" are
# not about time.

require_relative "../lib/claims"

PROSE_WORDS = /\b(today|currently|current|for now|so far|at the moment|as of|recently|nowadays|not yet|yet|now)\b(?!-)/i
COMMENT_WORDS = /\b(today|for now|not yet|at the moment|as of|recently|nowadays)\b(?!-)/i
ISSUE = %r{(?<![\w/])#\d+\b|issues/\d+\b}
CONJUNCTION = /(,|\band)\s*\z/

def unquoted(text) = text.gsub(/`[^`]*`/, "").gsub(/"[^"]*"/, "").gsub(/\]\([^)]*\)/, "]")

# The time words in text whose clause names no issue.
def unanchored(text, words)
  unquoted(text).split(/(?<=[.;:!?])\s+|\s+—\s+/).flat_map do |clause|
    next [] if clause.match?(ISSUE)

    clause.to_enum(:scan, words).filter_map do
      match = Regexp.last_match
      next if match[0].casecmp?("yet") && match.pre_match.match?(CONJUNCTION)

      match[0]
    end
  end
end

def report(where, text, found)
  <<~REPORT
    #{where}

      Says #{found.uniq.map { "\"#{it}\"" }.join(", ")}, which ages without notice:
        #{text.strip[0, 160]}

      Say what is true without a clock, or name the issue that ends the
      pending state in the same clause.
  REPORT
end

failures = []

every_claim.each do |path, text, number|
  words = path.end_with?(".md") ? PROSE_WORDS : COMMENT_WORDS
  found = unanchored(text, words)
  failures << report("#{path}:#{number}", text, found) unless found.empty?
end

manifest_entries.each do |upstream, entry|
  found = unanchored(entry["reason"].to_s, PROSE_WORDS)
  failures << report("spec/ruby_spec.yml: #{upstream}", entry["reason"].to_s, found) unless found.empty?
end

finish("time_words", failures, "the docs, comments, and manifest use no unanchored time words")
