# Writes generated sections. Run by script/docs/generate, which may name which
# ones to write; with no names, all of them.

require_relative "sections"

names = SECTIONS.map { |section| section[:name] }
wanted = ARGV.map { |argument| argument.delete_suffix(".rb") }
unknown = wanted - names

unless unknown.empty?
  warn "script/docs/generate: no generator named '#{unknown.first}'"
  warn "  available: #{names.join(" ")}"
  exit 1
end

selected = wanted.empty? ? SECTIONS : SECTIONS.select { |section| wanted.include?(section[:name]) }
changed = 0

selected.each do |section|
  path = "#{REPO}/#{section[:index]}"
  contents = read(path)
  rewritten = replace_generated_section(contents, section[:build].call)

  abort "#{relative(path)}: no #{GENERATED_BEGIN} … #{GENERATED_END} markers to write between" if rewritten.nil?

  if rewritten == contents
    puts "  same  #{relative(path)}"
    next
  end

  File.write(path, rewritten)
  changed += 1
  puts "  wrote #{relative(path)}"
end

puts changed.zero? ? "docs already current" : "docs updated — #{count(changed, "section")}"
