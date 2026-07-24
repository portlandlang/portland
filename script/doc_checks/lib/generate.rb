# Writes every generated section. Run by script/generate_docs.

require_relative "shared"
require_relative "generators"

changed = 0

GENERATED_SECTIONS.each do |section|
  path = "#{REPO}/#{section.fetch(:index)}"
  contents = read(path)
  rewritten = replace_generated_section(contents, section.fetch(:build).call)

  if rewritten.nil?
    abort "#{relative(path)}: no #{GENERATED_BEGIN} … #{GENERATED_END} markers to write between"
  end

  if rewritten == contents
    puts "  same  #{relative(path)}"
    next
  end

  File.write(path, rewritten)
  changed += 1
  puts "  wrote #{relative(path)}"
end

puts changed.zero? ? "docs already current" : "docs updated — #{changed} of #{GENERATED_SECTIONS.length}"
