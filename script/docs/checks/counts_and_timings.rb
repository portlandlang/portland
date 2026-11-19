# No counts and no exact timings in prose (#288; AGENT.md, "Writing that
# stays true").
#
# "~7,900 lines", "the spec's 136 examples", "31.5s of a 33s budget": each
# was true for one commit and then quietly wasn't. A count of files, lines,
# examples, rows, issues, or entries, and a timing given as an exact
# number, are flagged in the docs, the implementations' comments, and the
# manifest's reasons. An approximate timing that makes a point is fine and
# passes when it says so: "about 15s", "~6s", "under a minute".
#
# Declines where it cannot tell (docs/principles.md, never guess): text in
# backticks or double quotes is someone's output or code, not a claim, and
# a number glued to other characters ("UTF-16 rows", "10**400 row",
# "%-10s") is not a count. Spelled-out numbers are not read: "two files
# answering one name" describes a shape, and telling it from a stale tally
# would be a guess.

require_relative "../lib/claims"

NOUNS = /examples?|example runs?|files?|lines?|specs?|tests?|issues?|entries|entry|rows?|ADRs|checks?|PRs?|commits?/
COUNT = /(?<![\w\-*.,\/#~$])\d[\d,]*\s+(?:#{NOUNS})\b/
TIMING = /(?<![\w\-*.,\/#%])\d+(?:\.\d+)?\s?(?:ms|µs|ns|s|seconds?|minutes?|hours?)\b/
APPROXIMATE = /(?:about|around|roughly|nearly|almost|under|over|within|at most|at least|~)\s*\z/i

# Quoted and backticked spans are output or code, not claims.
def unquoted(text) = text.gsub(/`[^`]*`/, "").gsub(/"[^"]*"/, "")

def findings(text)
  plain = unquoted(text)
  counts = plain.scan(COUNT)
  timings = plain.to_enum(:scan, TIMING).filter_map do
    match = Regexp.last_match
    match[0] unless match.pre_match.match?(APPROXIMATE)
  end
  counts + timings
end

def report(where, text, found)
  <<~REPORT
    #{where}

      Names #{found.map { "\"#{it}\"" }.join(", ")}, which is true for one commit:
        #{text.strip[0, 160]}

      Say it without the number, or as an approximation if the size is the point.
  REPORT
end

failures = []

every_claim.each do |path, text, number|
  found = findings(text)
  failures << report("#{path}:#{number}", text, found) unless found.empty?
end

manifest_entries.each do |upstream, entry|
  found = findings(entry["reason"].to_s)
  failures << report("spec/ruby_spec.yml: #{upstream}", entry["reason"].to_s, found) unless found.empty?
end

finish("counts_and_timings", failures, "the docs, comments, and manifest name no counts or exact timings")
