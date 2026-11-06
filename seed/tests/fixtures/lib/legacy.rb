# A `.rb` file on the load path (ADR 0077): the extension is Ruby's, the
# grammar is Portland's, and `require "legacy"` finds it.

def legacy_greeting
  "hello from a .rb file"
end
