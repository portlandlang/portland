# What a reader takes as a claim, for the checks that hold prose to the code
# (#288): prose in the docs outside code fences, comments in the
# implementations, and the manifest's reasons.
#
# Decision records (docs/adr/), dated writing (docs/history/), and the
# CHANGELOG are history and are not read: what was true when they were
# written is what they should say.

require "yaml"
require_relative "shared"

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

# [text, line number] pairs: prose outside code fences, and comments in code.
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

# Every claim the prose and comment files make, as [path, text, line number].
def every_claim
  (prose_files + comment_files).flat_map do |path|
    claims(path).map { |text, number| [path, text, number] }
  end
end

# The manifest's entries, upstream file to entry, skipping comment-only keys.
def manifest_entries
  YAML.safe_load_file("#{REPO}/spec/ruby_spec.yml").select { |_upstream, entry| entry.is_a?(Hash) }
end
