# The least Ruby that lets a bench run under MRI unchanged, so script/bench
# can show a Ruby column beside the seed's (#25). This is not the polyfill
# gem (#36); it is the four spellings the benches use that Ruby lacks.
#
# - `mutable total = 0` parses in Ruby as `mutable(total = 0)`: the
#   assignment happens, and `mutable` has nothing left to do.
# - `failure(reason)` is nil here. A failure box is a value Portland's
#   or-guard treats as absent, and nil is the one value Ruby's `or` does
#   the same with; the reason is dropped, which a bench never reads.
# - `failure?` is the toolkit's predicate (ADR 0044): true of that nil,
#   false of everything else.
#
# A bench that needs more than this — struct patterns, the compiler itself —
# is skipped for Ruby with its reason, not shimmed further.

def mutable(bound) = bound

def failure(_reason) = nil

class Object
  def failure? = false
end

class NilClass
  def failure? = true
end
