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
    path == "CHANGELOG.md" || path.start_with?("docs/history/", "docs/adr/0") || path.include?("/fixtures/")
  end
end

def comment_files
  tracked(%w['seed/src/*' 'seed/tests/*.rs' 'compiler/*' 'spec/*.pdx' 'script/*']).reject do |path|
    path.start_with?("spec/ruby/") || path.include?("/fixtures/")
  end
end

# A comment's text: Rust's `//`, and elsewhere a `#` that starts the line or
# follows a space — so `"#{name}"` inside a string, or `#` inside a Rust
# string literal, is not mistaken for one.
def comment_text(path, line)
  return line[%r{//+\s?(.*)}, 1] if path.end_with?(".rs")

  line[/(?:\A|\s)#(?![!{])\s?(.*)/, 1]
end

# [text, line number] pairs: prose outside code fences and generated
# sections, and comments in code.
def claims(path)
  lines = read_lines("#{REPO}/#{path}")
  if path.end_with?(".md")
    inside = false
    generated = false
    lines.each_with_index.filter_map do |line, index|
      inside = !inside if line.lstrip.start_with?("```")
      generated = true if line.include?(GENERATED_BEGIN)
      generated = false if line.include?(GENERATED_END)
      [line, index + 1] unless inside || generated
    end
  else
    lines.each_with_index.filter_map do |line, index|
      comment = comment_text(path, line)
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
