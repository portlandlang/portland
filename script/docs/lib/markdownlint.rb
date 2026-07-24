# mdl (markdownlint) links
# gem repo:
#     https://github.com/markdownlint/markdownlint
# rules:
#     https://github.com/markdownlint/markdownlint/blob/main/docs/RULES.md
# configuration:
#     https://github.com/markdownlint/markdownlint/blob/main/docs/configuration.md
#     relevant .mdlrc
# styles:
#     https://github.com/markdownlint/markdownlint/blob/main/docs/creating_styles.md
#     relevant to this file!

# load all rules
all

# skip these rules/tags
# https://github.com/markdownlint/markdownlint/blob/main/docs/RULES.md

# allow long lines
exclude_rule "MD013"

# MD036 (emphasis used instead of a header) stays ON.
#
# It was briefly excluded: this repo's `**For:**` audience line and
# `**Summary:**` index line both sit under a heading, and when they were
# written as wholly-italic paragraphs — `_For: …_` — MD036 flagged them.
# Writing them as a bold label plus plain text keeps them visually distinct
# while putting them outside MD036's reach by construction, rather than by
# the accident of ending in ".,;:!?" which is what the rule actually skips.

# configure these rules (like .rubocop.yml)
# any rule in with `params` is configurable
# search here for which rules have `params`:
# https://github.com/markdownlint/markdownlint/blob/main/lib/mdl/rules.rb

# ensure that all headings are ATX style
# ATX headings are 1-6 leading octothorpes, example:
#     # This is an ATX H1
#     ## This is an ATX H2
rule "MD003", style: :atx

# ensure that all unordered lists start with a hyphen,
# not asterisks or pluses
rule "MD004", style: :dash

# indent nested listed with four spaces
rule "MD007", indent: 4

# allow ending heading with question mark
# default disallowed list is: ".,;:!?"
rule "MD026", punctuation: ".,;:!"

# ensure that all horizontal lists are hyphen style,
# not asterisks or hyphens with spaces
rule "MD035", style: "---"

# ensure that all code blocks use backtick fences, not indentation
# example:
# ```ruby
# ...
# ```
rule "MD046", style: :fenced
