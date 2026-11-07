# Onigmo conformance corpus

`cases.tsv` is Onigmo's own test suite, the `x2`, `x3`, and `n` cases of `testpy.py` at [k-takata/Onigmo](https://github.com/k-takata/Onigmo) master as of 2026-11-07, UTF-8 branch: 1,075 cases. Skipped on the way in: cases inside the UTF-16, UTF-32, Shift_JIS, and EUC-JP branches, cases that pass a syntax, option, search type, or start/end/g position (Ruby's regexp has no spelling for them), and five in other quoting. One line per case:

```text
testpy-line \t kind \t pattern-hex \t text-hex \t onigmo-expectation-json
```

`ruby_4_0_9.tsv` is what Ruby 4.0.9 answers for each case, `Regexp.new(pattern).match(text)` with byte offsets from `MatchData#byteoffset`, and is the ground truth the seed's engine is pinned against (ADR 0086: tie goes to Ruby). One line per case:

```text
testpy-line \t error \t sentence
testpy-line \t nomatch
testpy-line \t match \t start-end \t start-end|- ...    (group 0 first)
```

The case patterns and texts are Onigmo's, under its license:

```text
Onigmo (Oniguruma-mod) LICENSE
------------------------------

Copyright (c) 2002-2018  K.Kosako  <kkosako0@gmail.com>
Copyright (c) 2011-2019  K.Takata  <kentkt AT csc DOT jp>
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:
1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in the
   documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
SUCH DAMAGE.
```
