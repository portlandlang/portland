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

# allow a fully-italic line as a subtitle under a heading
#
# Two conventions in this repo are exactly the shape MD036 forbids: the
# `_For: …_` audience line every current doc carries, and the `_summary_`
# line the index generators read out of each ledger and history file.
# script/docs/checks/audience.rb *requires* the first of those, so the two
# checks would contradict each other.
#
# Note this rule only fires when the line does not end in ".,;:!?" — every
# audience line but ROADMAP's happens to end in a period, so the conflict
# was latent rather than loud.
exclude_rule "MD036"

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
