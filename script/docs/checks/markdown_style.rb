# Markdown style, via mdl. Rules and their reasoning live in
# ../lib/markdownlint.rb; the .mdlrc at the repo root points mdl at it.
#
# `git_recurse` in that .mdlrc means mdl lints **git-tracked files only**. A
# brand-new file is invisible to it until `git add`, which is fine for the
# pre-commit hook (staging is what the hook runs against) but does mean a
# scratch file you have not added is silently unlinted.
#
# This is the one check that is not bespoke. mdl covers the generic formatting
# rules — heading style, list markers, trailing whitespace, hard tabs, code
# fence languages — so the other checks in this directory can stay about
# things only this project cares about.
#
# It does not cover the rule we care most about: mdl has no check for
# *hard-wrapped* prose. MD013 is a ceiling on line length and nothing in its
# 39 rules is the floor, so AGENT.md holds that one for a human.

require_relative "../lib/shared"

output = IO.popen(%w[bundle exec mdl .], chdir: REPO, err: [:child, :out], &:read)
clean = $?.success?

version = read("#{REPO}/Gemfile.lock")[/^ {4}mdl \(([^)]+)\)/, 1] || "unknown"

failures = []

unless clean
  failures << <<~REPORT
    #{output.strip}

      Rules are configured in script/docs/lib/markdownlint.rb, and every
      non-default setting there says why. A kramdown warning is a parse
      ambiguity — usually unescaped brackets — rather than a style rule.
  REPORT
end

finish("markdown_style", failures, "mdl #{version}")
