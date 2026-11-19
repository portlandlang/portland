# A claim that waits on an issue is stale once the issue closes (#288).
#
# ADRs 0002, 0004, and 0011 said `together` was unbuilt for four months
# after it was built, and manifest reasons said "no Rational" long after
# #198 built it (#287). Each such claim names, or could name, the issue that
# ends it; this check asks GitHub which issues are closed and flags every
# claim still waiting on one — "waits on #N", "until #N", "#N tracks it" —
# in the docs, the manifest's reasons, and the implementations' comments.
#
# It also holds the manifest to its own word: a `partial_planned` entry is
# planned on an issue, so it must cite at least one that is open.
#
# Decision records (docs/adr/), dated writing (docs/history/), and the
# CHANGELOG are history and are not read: a closed issue is what they should
# say. Where `gh` cannot answer — no network, no login — the check declines
# rather than guessing, and says so in its summary.

require "json"
require "yaml"
require_relative "../lib/shared"

# Words that put a claim in waiting on what follows them.
WAITING = /\b(waits? (on|for)|waiting (on|for)|until|blocked (on|by)|pending|deferred to|owed to|tracked (in|by)|rides with|depends on)\b/i
# An issue, as `#N` or a link to one.
ISSUE = %r{(?<![\w/])#(\d+)\b|portland/issues/(\d+)\b}
# Words after an issue that say it is still deciding or building something.
STILL_OPEN = /\A\)?\]?(\([^)]*\))?\s*(tracks|will (decide|build|settle)|decides|is open|is the question)\b/i

def closed_issue_numbers
  output = IO.popen(%w[gh issue list --repo portlandlang/portland --state closed --limit 5000 --json number],
                    chdir: REPO, err: File::NULL, &:read)
  return nil unless $?.success?

  JSON.parse(output).map { it["number"] }.to_set
rescue Errno::ENOENT, JSON::ParserError
  nil
end

def issues_in(text) = text.scan(ISSUE).map { |short, long| Integer(short || long) }

# The issues a line says it is waiting on: those after a waiting word in the
# same clause, and those followed by a still-open verb.
def waited_on(text)
  waiting = []
  text.scan(/#{WAITING}([^.;]{0,80})/) { waiting.concat(issues_in(Regexp.last_match[0])) }
  text.to_enum(:scan, ISSUE).each do
    match = Regexp.last_match
    waiting << Integer(match[1] || match[2]) if match.post_match.match?(STILL_OPEN)
  end
  waiting.uniq
end

def tracked(patterns)
  Dir.chdir(REPO) { `git ls-files -z -- #{patterns.join(" ")}`.split("\0") }
end

def prose_files
  tracked(["'*.md'"]).reject do |path|
    path == "CHANGELOG.md" || path.start_with?("docs/history/", "docs/adr/0")
  end
end

def comment_files
  tracked(%w['seed/src/*' 'seed/tests/*.rs' 'compiler/*' 'spec/*.pdx' 'script/*']).reject do |path|
    path.start_with?("spec/ruby/") || path.include?("/fixtures/")
  end
end

# [text, line number] pairs a reader takes as claims: prose outside code
# fences, and comments in code.
def claims(path)
  lines = read_lines("#{REPO}/#{path}")
  if path.end_with?(".md")
    inside = false
    lines.each_with_index.filter_map do |line, index|
      inside = !inside if line.lstrip.start_with?("```")
      [line, index + 1] unless inside
    end
  else
    lines.each_with_index.filter_map do |line, index|
      comment = line[%r{(?://|#)(?!!)(.*)}, 1]
      [comment, index + 1] if comment
    end
  end
rescue ArgumentError
  []
end

closed = closed_issue_numbers
if closed.nil?
  finish("closed_issue_claims", [], "skipped — `gh` could not list closed issues, so nothing was checked")
end

failures = []

(prose_files + comment_files).each do |path|
  claims(path).each do |text, number|
    waited_on(text).select { closed.include?(it) }.each do |issue|
      failures << <<~REPORT
        #{path}:#{number}

          Waits on ##{issue}, which is closed:
            #{text.strip[0, 160]}

          Bring the claim up to date with what ##{issue} did (script/mentions #{issue}).
      REPORT
    end
  end
end

manifest = YAML.safe_load_file("#{REPO}/spec/ruby_spec.yml")
manifest.each do |upstream, entry|
  next unless entry.is_a?(Hash)

  waited_on(entry["reason"].to_s).select { closed.include?(it) }.each do |issue|
    failures << <<~REPORT
      spec/ruby_spec.yml: #{upstream}

        Waits on ##{issue}, which is closed:
          #{entry["reason"].to_s.strip[0, 160]}

        Bring the reason up to date with what ##{issue} did (script/mentions #{issue}).
    REPORT
  end

  cited = issues_in("#{entry["reason"]} #{entry["link"]}")
  if entry["status"] == "partial_planned" && cited.empty?
    failures << <<~REPORT
      spec/ruby_spec.yml: #{upstream}

        partial_planned names no issue. A plan needs the open issue that
        carries it, in the reason or the link.
    REPORT
  end
  next unless entry["status"] == "partial_planned"
  next if cited.empty? || cited.any? { !closed.include?(it) }

  failures << <<~REPORT
    spec/ruby_spec.yml: #{upstream}

      partial_planned, but every issue it cites is closed (#{cited.map { "##{it}" }.join(", ")}).
      Port what they built, then say what still stays upstream and why,
      naming the open issue that carries what is left.
  REPORT
end

finish("closed_issue_claims", failures, "#{count(closed.size, "closed issue")} against the docs, comments, and manifest")
