# Every ```ruby block in the docs parses as Portland (#35).
#
# Writing docs/language.md produced three constructs that do not exist, all
# from memory in one sitting: an endless method (`def integer? = …`), a
# one-line `if/then/else`, and a ternary. None would have been caught by
# reading. A doc that teaches syntax the language does not have is worse than
# a missing doc.
#
# Parsing is the right depth. `pdx --parse` does not evaluate, so a sample may
# reference `lookup(id)` or `article` without defining them — which is how
# most illustrative snippets are written — while invented *syntax* still
# fails.
#
# Blocks are tagged ```ruby rather than ```portland because that is what
# GitHub highlights, and Portland is Ruby-shaped on purpose.
#
# ## Opting out
#
# Put an HTML comment immediately above the fence, with a reason:
#
#     <!-- not-portland: Ruby, shown for contrast -->
#
# The reason is required. Three kinds are legitimate and all appear in this
# repo: Ruby quoted for comparison (the whole `docs/ruby/` ledger), Portland
# that is decided but not yet built (`together`, symbols), and deliberately
# invalid Portland (the never-guess errors, which exist to be refused).
# Requiring the reason keeps an exemption from being a silent shrug.
#
# Deliberately absent: flagging a *stale* exemption — a marked block that has
# since started parsing, which is what will happen to the six `together`
# blocks the day concurrency ships. It looks free but it is not: a block
# marked "Ruby, shown for contrast" may be valid Portland syntax by
# coincidence, since Portland is Ruby-shaped on purpose, and the check would
# call that a stale exemption every time. A check that cries wolf is worse
# than no check. Removing the markers belongs to whoever builds the feature.

require_relative "../lib/shared"

MARKER = /\A\s*<!--\s*not-portland:\s*(.+?)\s*-->/

# Fences are indented inside list items, so the opening indent is stripped
# from the body — that indent is markdown's, not the sample's.
def ruby_blocks(path)
  blocks = []
  open_line = nil
  indent = 0
  body = []

  read_lines(path).each_with_index do |line, index|
    if open_line.nil?
      next unless line.match?(/\A\s*```ruby\s*\z/)

      open_line = index + 1
      indent = line[/\A\s*/].length
      body = []
    elsif line.match?(/\A\s*```\s*\z/)
      blocks << { line: open_line, source: body.join }
      open_line = nil
    else
      body << line.sub(/\A {0,#{indent}}/, "")
    end
  end

  blocks
end

# The marker sits above the fence, with a blank line between them — MD031
# wants fences surrounded by blank lines, so the two cannot be adjacent.
def exempt?(path, line)
  above = read_lines(path)[0...(line - 1)].reverse.find { |candidate| !candidate.strip.empty? }
  above.to_s.match(MARKER)
end

# `script/test` runs cargo before the doc checks, so the binary is normally
# fresh; building here keeps `script/docs/check` usable on its own.
def pdx_binary
  binary = "#{REPO}/target/debug/pdx"
  return binary if File.executable?(binary)

  ENV["PATH"] = "/opt/homebrew/opt/rustup/bin:#{ENV.fetch("PATH", "")}"
  Dir.chdir(REPO) { system("cargo", "build", "--quiet", "--bin", "pdx") }
  abort "cannot find or build target/debug/pdx — run script/test once" unless File.executable?(binary)

  binary
end

pdx = pdx_binary
sample = "#{REPO}/target/doc_sample.pdx"
failures = []
checked = 0

markdown_files.each do |path|
  ruby_blocks(path).each do |block|
    next if exempt?(path, block[:line])

    checked += 1
    File.write(sample, block[:source])
    parsed = system(pdx, "--parse", sample, out: File::NULL, err: File::NULL)
    next if parsed

    error = IO.popen([pdx, "--parse", sample], err: [:child, :out], &:read)
    failures << <<~REPORT
      #{relative(path)}:#{block[:line]}

        This ```ruby block does not parse as Portland:

      #{error.lines.grep(/panicked|^[a-z]/).first(2).map { |line| "    #{line.strip}" }.join("\n")}

        Either fix the sample, or mark it with a reason on the line above
        the fence:

          <!-- not-portland: Ruby, shown for contrast -->
    REPORT
  end
end

File.delete(sample) if File.exist?(sample)

finish("code_samples", failures, count(checked, "block"))
