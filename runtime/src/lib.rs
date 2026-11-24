//! The runtime every native Portland program is linked with (ADR 0095).
//!
//! The compiler writes a program as LLVM IR, and what that IR cannot
//! sensibly spell inline lives here: the memory behind a value that needs
//! the heap, and the operations on it. It is the floor's first real piece,
//! built as a static library and handed to `clang` by `script/build`.
//!
//! Every function here is called from compiled Portland through the C
//! calling convention and is named `portland_…`. A panic in one aborts the
//! program: an `extern "C"` function never unwinds into its caller.
//!
//! Nothing here is the seed's code. The two share crates through the
//! workspace, never source: the Unicode crates, and `portland-source-form`,
//! which writes a String back out as source for both (#507).

/// A String as a native program holds it: a pointer to this header, with
/// the text's UTF-8 bytes straight after it.
///
/// It is the plain layout on purpose. Nothing a Portland program can
/// observe depends on it, so it can be replaced when a measurement says to.
#[repr(C)]
pub struct StringBlock {
    /// How many names hold this String, or `LITERAL` for one written in
    /// the program's source, which is part of the binary and never freed.
    count: i64,
    /// The text's length in bytes.
    length: i64,
}

/// The count of a String literal: it lives in the binary for as long as
/// the program runs.
pub const LITERAL: i64 = -1;

unsafe extern "C" {
    /// libc's standard output stream, as macOS names it. Compiled Portland
    /// writes Integers with libc's `printf`, so text written here has to go
    /// through the same stream to come out in order.
    static __stdoutp: *mut libc::FILE;
}

/// The bytes of a String's text.
///
/// # Safety
/// `text` points at a `StringBlock` followed by `length` readable bytes.
unsafe fn bytes_of<'text>(text: *const StringBlock) -> &'text [u8] {
    unsafe {
        let length = (*text).length as usize;
        std::slice::from_raw_parts(text.add(1).cast::<u8>(), length)
    }
}

/// `puts` of one String: its text and a line ending, or the text alone
/// where it already ends in one, as Ruby's `puts` writes it.
///
/// # Safety
/// `text` points at a live String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_puts_string(text: *const StringBlock) {
    unsafe {
        debug_assert!(
            (*text).count == LITERAL || (*text).count > 0,
            "puts of a String that was already freed"
        );
        let bytes = bytes_of(text);
        libc::fwrite(bytes.as_ptr().cast(), 1, bytes.len(), __stdoutp);
        if bytes.last() != Some(&b'\n') {
            libc::fputc(i32::from(b'\n'), __stdoutp);
        }
    }
}

/// How many Strings made at run time are alive. Kept only by the debug
/// build, which is the one that checks the compiler's counting.
static LIVE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// The Strings made at run time and not yet freed: zero in a release build,
/// which does not keep the tally.
pub fn live_strings() -> i64 {
    LIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The memory a String of `length` bytes takes: its header and its text.
fn layout_of(length: usize) -> std::alloc::Layout {
    std::alloc::Layout::from_size_align(
        std::mem::size_of::<StringBlock>() + length,
        std::mem::align_of::<StringBlock>(),
    )
    .expect("a String's size fits the address space")
}

/// A new String holding the given pieces end to end, held once: by
/// whoever asked for it.
fn string_from(pieces: &[&[u8]]) -> *mut StringBlock {
    let length: usize = pieces.iter().map(|piece| piece.len()).sum();
    unsafe {
        let block = std::alloc::alloc(layout_of(length)).cast::<StringBlock>();
        if block.is_null() {
            std::alloc::handle_alloc_error(layout_of(length));
        }
        block.write(StringBlock {
            count: 1,
            length: length as i64,
        });
        let mut next = block.add(1).cast::<u8>();
        for piece in pieces {
            std::ptr::copy_nonoverlapping(piece.as_ptr(), next, piece.len());
            next = next.add(piece.len());
        }
        if cfg!(debug_assertions) {
            LIVE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        block
    }
}

/// `left + right`: a new String, held once.
///
/// # Safety
/// Both point at live Strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_concat(
    left: *const StringBlock,
    right: *const StringBlock,
) -> *mut StringBlock {
    unsafe { string_from(&[bytes_of(left), bytes_of(right)]) }
}

/// An Integer as its digits, for interpolation: a new String, held once.
#[unsafe(no_mangle)]
pub extern "C" fn portland_string_from_integer(number: i64) -> *mut StringBlock {
    string_from(&[number.to_string().as_bytes()])
}

/// Whether two Strings are equal, which for a String is canonical
/// equivalence (ADR 0038): `é` as one code point equals `é` as `e` and a
/// combining accent. Equal bytes are equal Strings, and are the short
/// path; otherwise the two are compared in normalized form, by the same
/// crate the seed compares with.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_equal(
    left: *const StringBlock,
    right: *const StringBlock,
) -> bool {
    use unicode_normalization::UnicodeNormalization;
    unsafe {
        let (first, second) = (bytes_of(left), bytes_of(right));
        if first == second {
            return true;
        }
        let first = std::str::from_utf8_unchecked(first);
        let second = std::str::from_utf8_unchecked(second);
        first.nfc().eq(second.nfc())
    }
}

/// Where one String stands against another: before it, the same, or after
/// it, as less than nothing, nothing, or more. Strings order by their
/// canonical text (ADR 0054), code point by code point, by the same crate
/// the seed orders with; equal bytes and all-ASCII text take the short
/// paths, as in the seed.
///
/// # Safety
/// Both point at live Strings, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_order(
    left: *const StringBlock,
    right: *const StringBlock,
) -> i64 {
    use unicode_normalization::UnicodeNormalization;
    unsafe {
        let (first, second) = (bytes_of(left), bytes_of(right));
        let ordering = if first == second {
            std::cmp::Ordering::Equal
        } else if first.is_ascii() && second.is_ascii() {
            first.cmp(second)
        } else {
            let first = std::str::from_utf8_unchecked(first);
            let second = std::str::from_utf8_unchecked(second);
            first.nfc().cmp(second.nfc())
        };
        ordering as i64
    }
}

/// One more name holds this String. A literal is not counted, and neither
/// is the absence of a String: a local not yet bound.
///
/// An array is retained through this too: every counted block begins with
/// its count, and a retain reads nothing else.
///
/// # Safety
/// `text` is null or points at a live String or a live array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_retain(text: *mut StringBlock) {
    unsafe {
        if text.is_null() || (*text).count == LITERAL {
            return;
        }
        (*text).count += 1;
    }
}

/// One name fewer holds this String, and the last one frees it. A count
/// that would go below zero means the compiler released a String it did
/// not hold; the debug build says so and stops.
///
/// # Safety
/// `text` is null or points at a String that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_release(text: *mut StringBlock) {
    unsafe {
        if text.is_null() || (*text).count == LITERAL {
            return;
        }
        if cfg!(debug_assertions) && (*text).count <= 0 {
            refuse(
                "a String was released more times than it was held — the native build counted wrong (#465)",
            );
        }
        (*text).count -= 1;
        if (*text).count == 0 {
            let length = (*text).length as usize;
            if cfg!(debug_assertions) {
                // Left readable as freed, so a second release is caught
                // above and never frees the block twice.
                LIVE.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
            std::alloc::dealloc(text.cast(), layout_of(length));
        }
    }
}

/// The end of a program that ran to its last line: in the debug build,
/// every String and every array made at run time must have been freed.
/// One that was not means the compiler missed a release.
#[unsafe(no_mangle)]
pub extern "C" fn portland_exit_check() {
    if cfg!(debug_assertions) && live_strings() != 0 {
        let count = live_strings();
        let noun = if count == 1 {
            "String was"
        } else {
            "Strings were"
        };
        refuse(&format!(
            "{count} {noun} still held when the program ended — the native build missed a release (#465)"
        ));
    }
    if cfg!(debug_assertions) && live_structs() != 0 {
        let count = live_structs();
        let noun = if count == 1 {
            "struct was"
        } else {
            "structs were"
        };
        refuse(&format!(
            "{count} {noun} still held when the program ended — the native build missed a release (#532)"
        ));
    }
    if cfg!(debug_assertions) && live_arrays() != 0 {
        let count = live_arrays();
        let noun = if count == 1 {
            "array was"
        } else {
            "arrays were"
        };
        refuse(&format!(
            "{count} {noun} still held when the program ended — the native build missed a release (#497)"
        ));
    }
}

/// An array as a native program holds it (ADR 0096): a pointer to this
/// header, with its elements straight after it, one word each.
///
/// The plain layout, as a String's is. Values never mutate, so an array
/// that two names hold is never changed under either: `<<` grows an array
/// held once where it is, and copies one held elsewhere first.
#[repr(C)]
pub struct ArrayBlock {
    /// How many names hold this array, or `LITERAL` for the empty array
    /// every `[]` starts as, which is part of the binary.
    count: i64,
    /// How many elements it holds.
    length: i64,
    /// How many it has room for before it must move.
    capacity: i64,
    /// What a slot holds, which says what freeing the array owes its
    /// elements.
    kind: i64,
}

/// The kinds of slot. An Integer and a Boolean are the value itself, a
/// plain word with nothing to release. A String and an array are counted
/// pointers, which the array holds once each.
pub const INTEGERS: i64 = 0;
pub const BOOLEANS: i64 = 1;
pub const STRINGS: i64 = 2;
pub const ARRAYS: i64 = 3;
/// A struct, which is counted too (#532).
pub const STRUCTS: i64 = 4;

/// One word, as its kind says to hold it: a holder gained or lost where
/// it is a counted pointer, and nothing where it is the value itself.
///
/// # Safety
/// `word` is a value of that kind, and a pointer among them is null or
/// points at a block that has not been freed.
unsafe fn hold_word(kind: i64, word: i64, held: bool) {
    unsafe {
        match (kind, held) {
            (STRINGS, false) => portland_release(word as *mut StringBlock),
            (ARRAYS, false) => portland_array_release(word as *mut ArrayBlock),
            (STRUCTS, false) => portland_struct_release(word as *mut StructBlock),
            (STRINGS | ARRAYS | STRUCTS, true) => portland_retain(word as *mut StringBlock),
            _ => {}
        }
    }
}

/// One word in its source form, as its kind says to write it.
///
/// # Safety
/// `word` is a value of that kind, and a pointer among them points at a
/// live block.
unsafe fn word_source(kind: i64, word: i64) -> String {
    unsafe {
        match kind {
            BOOLEANS => (word != 0).to_string(),
            STRINGS => portland_source_form::quoted(std::str::from_utf8_unchecked(bytes_of(
                word as *const StringBlock,
            ))),
            ARRAYS => array_source(word as *mut ArrayBlock),
            STRUCTS => struct_source(word as *mut StructBlock),
            _ => word.to_string(),
        }
    }
}

/// Every element of an array of counted pointers gains a holder, or
/// loses one. An array of plain words owes its elements nothing.
///
/// # Safety
/// `array` points at an array whose elements are what its kind says.
unsafe fn each_counted(array: *mut ArrayBlock, held: bool) {
    unsafe {
        let kind = (*array).kind;
        if kind != STRINGS && kind != ARRAYS && kind != STRUCTS {
            return;
        }
        for index in 0..(*array).length as usize {
            hold_word(kind, elements_of(array).add(index).read(), held);
        }
    }
}

/// How many arrays made at run time are alive, kept by the debug build.
static LIVE_ARRAYS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// The arrays made at run time and not yet freed: zero in a release
/// build, which does not keep the tally.
pub fn live_arrays() -> i64 {
    LIVE_ARRAYS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The memory an array with room for `capacity` elements takes.
fn array_layout(capacity: usize) -> std::alloc::Layout {
    std::alloc::Layout::from_size_align(
        std::mem::size_of::<ArrayBlock>() + capacity * std::mem::size_of::<i64>(),
        std::mem::align_of::<ArrayBlock>(),
    )
    .expect("an array's size fits the address space")
}

/// Where an array's elements begin.
///
/// # Safety
/// `array` points at an `ArrayBlock`.
unsafe fn elements_of(array: *mut ArrayBlock) -> *mut i64 {
    unsafe { array.add(1).cast::<i64>() }
}

/// A new, empty array with room for `capacity` elements, held once.
fn array_with(kind: i64, capacity: usize) -> *mut ArrayBlock {
    unsafe {
        let block = std::alloc::alloc(array_layout(capacity)).cast::<ArrayBlock>();
        if block.is_null() {
            std::alloc::handle_alloc_error(array_layout(capacity));
        }
        block.write(ArrayBlock {
            count: 1,
            length: 0,
            capacity: capacity as i64,
            kind,
        });
        if cfg!(debug_assertions) {
            LIVE_ARRAYS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        block
    }
}

/// A new, empty array with room for what a literal is about to put in
/// it, held once.
#[unsafe(no_mangle)]
pub extern "C" fn portland_array_new(kind: i64, capacity: i64) -> *mut ArrayBlock {
    array_with(kind, capacity.max(0) as usize)
}

/// The least room a grown array is given.
const SMALLEST_ROOM: usize = 4;

/// An array ready to take one more element, held once, in exchange for
/// the caller's hold on `array`. One held once is the same array, moved
/// if it was full. One held elsewhere, the empty literal among them, is
/// left as it was and answered by a copy.
///
/// # Safety
/// `array` points at a live array, and the caller holds it.
unsafe fn with_room(array: *mut ArrayBlock, kind: i64) -> *mut ArrayBlock {
    unsafe {
        let length = (*array).length as usize;
        let capacity = (*array).capacity as usize;
        if (*array).count == 1 {
            if length < capacity {
                return array;
            }
            let roomier = (capacity * 2).max(SMALLEST_ROOM);
            let moved = std::alloc::realloc(
                array.cast(),
                array_layout(capacity),
                array_layout(roomier).size(),
            )
            .cast::<ArrayBlock>();
            if moved.is_null() {
                std::alloc::handle_alloc_error(array_layout(roomier));
            }
            (*moved).capacity = roomier as i64;
            return moved;
        }
        // The copy holds each element as the original does.
        let copy = array_with(kind, (length * 2).max(SMALLEST_ROOM));
        std::ptr::copy_nonoverlapping(elements_of(array), elements_of(copy), length);
        (*copy).length = length as i64;
        each_counted(copy, true);
        portland_array_release(array);
        copy
    }
}

/// `array << value`: the array with the value after its last element,
/// held once, in exchange for the caller's hold on `array`. The value is
/// one word: an Integer, a Boolean, or a counted pointer, whose hold the
/// array takes over. `kind` says which, for an array made here.
///
/// # Safety
/// `array` is null or points at a live array of that kind which the
/// caller holds, and a counted `value` points at a live block the caller
/// holds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_append(
    array: *mut ArrayBlock,
    value: i64,
    kind: i64,
) -> *mut ArrayBlock {
    unsafe {
        // A local on a path that never bound it holds nothing, which
        // grows as the empty array does.
        let array = if array.is_null() {
            array_with(kind, SMALLEST_ROOM)
        } else {
            array
        };
        let grown = with_room(array, kind);
        let length = (*grown).length as usize;
        elements_of(grown).add(length).write(value);
        (*grown).length = (length + 1) as i64;
        grown
    }
}

/// One name fewer holds this array, and the last one frees it. The
/// empty literal is not counted, and neither is a local not yet bound.
///
/// # Safety
/// `array` is null or points at an array that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_release(array: *mut ArrayBlock) {
    unsafe {
        if array.is_null() || (*array).count == LITERAL {
            return;
        }
        if cfg!(debug_assertions) && (*array).count <= 0 {
            refuse(
                "an array was released more times than it was held — the native build counted wrong (#497)",
            );
        }
        (*array).count -= 1;
        if (*array).count == 0 {
            each_counted(array, false);
            if cfg!(debug_assertions) {
                // Left readable as freed, as a String is, so a second
                // release is caught above.
                LIVE_ARRAYS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
            std::alloc::dealloc(array.cast(), array_layout((*array).capacity as usize));
        }
    }
}

/// A refusal the program itself earned, as the seed words it: what was
/// written so far, then the sentence on standard error, and the status
/// the seed exits with.
fn refuse_program(sentence: &str) -> ! {
    unsafe {
        libc::fflush(__stdoutp);
    }
    eprintln!("{sentence}");
    std::process::exit(1);
}

/// `left / right` between Integers, rounding toward negative infinity as
/// Ruby's does. A zero divisor and the one quotient that does not fit
/// refuse in the seed's sentences (ADR 0070).
#[unsafe(no_mangle)]
pub extern "C" fn portland_integer_divide(left: i64, right: i64) -> i64 {
    if right == 0 {
        refuse_program("divided by 0");
    }
    let Some(quotient) = left.checked_div(right) else {
        refuse_program(&format!("{left} / {right} overflows the 64-bit integers"));
    };
    // Rust rounds toward zero; where the signs differ and the division is
    // inexact, the floor is one below.
    if left % right != 0 && ((left < 0) != (right < 0)) {
        quotient - 1
    } else {
        quotient
    }
}

/// `left % right` between Integers: the remainder takes the divisor's
/// sign, as Ruby's does, so it pairs with the division above.
#[unsafe(no_mangle)]
pub extern "C" fn portland_integer_modulo(left: i64, right: i64) -> i64 {
    if right == 0 {
        refuse_program("divided by 0");
    }
    // The smallest Integer over -1 has no remainder, and asking the
    // machine for it traps.
    let remainder = left.checked_rem(right).unwrap_or(0);
    if remainder != 0 && ((remainder < 0) != (right < 0)) {
        remainder + right
    } else {
        remainder
    }
}

/// `number.abs`, which the smallest Integer has no answer for.
#[unsafe(no_mangle)]
pub extern "C" fn portland_integer_abs(number: i64) -> i64 {
    number
        .checked_abs()
        .unwrap_or_else(|| refuse_program(&format!("{number}.abs overflows the 64-bit integers")))
}

/// `number.succ`, one past the largest Integer refusing.
#[unsafe(no_mangle)]
pub extern "C" fn portland_integer_succ(number: i64) -> i64 {
    number
        .checked_add(1)
        .unwrap_or_else(|| refuse_program(&format!("{number}.succ overflows the 64-bit integers")))
}

/// `number.pred`, one before the smallest Integer refusing.
#[unsafe(no_mangle)]
pub extern "C" fn portland_integer_pred(number: i64) -> i64 {
    number
        .checked_sub(1)
        .unwrap_or_else(|| refuse_program(&format!("{number}.pred overflows the 64-bit integers")))
}

/// A String's length, which counts characters, and a character is a
/// grapheme cluster (ADR 0038): a flag is one, and so is a letter with a
/// combining accent. Counted by the crate the seed counts with.
///
/// # Safety
/// `text` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_length(text: *const StringBlock) -> i64 {
    use unicode_segmentation::UnicodeSegmentation;
    unsafe {
        std::str::from_utf8_unchecked(bytes_of(text))
            .graphemes(true)
            .count() as i64
    }
}

/// `-number`, which the smallest Integer has no answer for.
#[unsafe(no_mangle)]
pub extern "C" fn portland_integer_negate(number: i64) -> i64 {
    number
        .checked_neg()
        .unwrap_or_else(|| refuse_program(&format!("-({number}) overflows the 64-bit integers")))
}

/// A Boolean written out, for `puts` and interpolation: a new String,
/// held once.
#[unsafe(no_mangle)]
pub extern "C" fn portland_string_from_boolean(value: bool) -> *mut StringBlock {
    string_from(&[if value { b"true" } else { b"false" }])
}

/// `inspect` of a String: its source form, as a new String held once.
/// The absence of a String, a maybe that is nil, is answered by absence.
///
/// # Safety
/// `text` is null or points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_string_inspect(text: *const StringBlock) -> *mut StringBlock {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        string_from(&[
            portland_source_form::quoted(std::str::from_utf8_unchecked(bytes_of(text))).as_bytes(),
        ])
    }
}

/// An array's source form, each element written by the kind on its
/// block, an array inside an array by its own.
///
/// # Safety
/// `array` points at a live array whose elements are what its kind says.
unsafe fn array_source(array: *mut ArrayBlock) -> String {
    unsafe {
        let mut written = Vec::with_capacity((*array).length as usize);
        for index in 0..(*array).length as usize {
            written.push(word_source(
                (*array).kind,
                elements_of(array).add(index).read(),
            ));
        }
        format!("[{}]", written.join(", "))
    }
}

/// An array written out, which for an array is its source form whether
/// `puts`, `inspect`, or interpolation asks: a new String, held once.
/// The absence of an array is answered by absence.
///
/// # Safety
/// `array` is null or points at a live array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_array_to_string(array: *mut ArrayBlock) -> *mut StringBlock {
    if array.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { string_from(&[array_source(array).as_bytes()]) }
}

/// What the compiler writes into the binary for each struct a program
/// declares (#532): its name, how many fields it has, and after this
/// header, for each field in order, the kind of slot it is and then its
/// name. It is what lets the runtime free a struct and write one out
/// without the compiler saying how each time.
#[repr(C)]
pub struct StructShape {
    name: *const StringBlock,
    fields: i64,
}

/// A struct as a native program holds it: a pointer to this header, with
/// its fields straight after it, one word each. The plain layout, as a
/// String's and an array's are. A struct never changes, so it is never
/// copied to be changed: `with` makes a new one.
#[repr(C)]
pub struct StructBlock {
    /// How many names hold this struct.
    count: i64,
    /// What struct it is.
    shape: *const StructShape,
}

/// How many structs made at run time are alive, kept by the debug build.
static LIVE_STRUCTS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// The structs made at run time and not yet freed: zero in a release
/// build, which does not keep the tally.
pub fn live_structs() -> i64 {
    LIVE_STRUCTS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The memory a struct takes whose fields are this many words.
fn struct_layout(words: usize) -> std::alloc::Layout {
    std::alloc::Layout::from_size_align(
        std::mem::size_of::<StructBlock>() + words * std::mem::size_of::<i64>(),
        std::mem::align_of::<StructBlock>(),
    )
    .expect("a struct's size fits the address space")
}

/// The kind of a shape's field, and its name: the two words written for
/// it after the shape's header.
///
/// # Safety
/// `shape` points at a shape with more than `index` fields.
unsafe fn field_of(shape: *const StructShape, index: usize) -> (i64, *const StringBlock) {
    unsafe {
        let words = shape.add(1).cast::<i64>();
        (
            words.add(index * 2).read(),
            words.add(index * 2 + 1).read() as *const StringBlock,
        )
    }
}

/// Added to a field's kind where the field is a maybe (#553): such a field
/// takes two words, whether it is there and then its value, where every
/// other takes one.
pub const MAYBE: i64 = 16;

/// How many words a field of this kind takes in its struct.
fn words_of(kind: i64) -> usize {
    if kind >= MAYBE { 2 } else { 1 }
}

/// How many words a struct of this shape takes for its fields.
///
/// # Safety
/// `shape` points at a shape the compiler wrote.
unsafe fn struct_words(shape: *const StructShape) -> usize {
    unsafe {
        (0..(*shape).fields as usize)
            .map(|index| words_of(field_of(shape, index).0))
            .sum()
    }
}

/// One field let go of as its kind says: a maybe's value only where it
/// is there.
///
/// # Safety
/// `slot` points at a field of that kind in a struct that is not freed.
unsafe fn release_field(kind: i64, slot: *mut i64) {
    unsafe {
        if kind >= MAYBE {
            if slot.read() != 0 {
                hold_word(kind - MAYBE, slot.add(1).read(), false);
            }
            return;
        }
        hold_word(kind, slot.read(), false);
    }
}

/// One field in its source form: a maybe that is not there is `nil`.
///
/// # Safety
/// `slot` points at a field of that kind in a live struct.
unsafe fn field_source(kind: i64, slot: *mut i64) -> String {
    unsafe {
        if kind >= MAYBE {
            if slot.read() == 0 {
                return "nil".to_string();
            }
            return word_source(kind - MAYBE, slot.add(1).read());
        }
        word_source(kind, slot.read())
    }
}

/// Where a struct's fields begin.
///
/// # Safety
/// `record` points at a `StructBlock`.
unsafe fn fields_of(record: *mut StructBlock) -> *mut i64 {
    unsafe { record.add(1).cast::<i64>() }
}

/// A new struct of the given shape, held once, its fields for the
/// compiled program to fill before anything reads them.
///
/// # Safety
/// `shape` points at a shape the compiler wrote.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_struct_new(shape: *const StructShape) -> *mut StructBlock {
    unsafe {
        let words = struct_words(shape);
        let block = std::alloc::alloc_zeroed(struct_layout(words)).cast::<StructBlock>();
        if block.is_null() {
            std::alloc::handle_alloc_error(struct_layout(words));
        }
        block.write(StructBlock { count: 1, shape });
        if cfg!(debug_assertions) {
            LIVE_STRUCTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        block
    }
}

/// One name fewer holds this struct, and the last one frees it, letting
/// go of each field it holds. A local not yet bound is not counted.
///
/// # Safety
/// `record` is null or points at a struct that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_struct_release(record: *mut StructBlock) {
    unsafe {
        if record.is_null() {
            return;
        }
        if cfg!(debug_assertions) && (*record).count <= 0 {
            refuse(
                "a struct was released more times than it was held — the native build counted wrong (#532)",
            );
        }
        (*record).count -= 1;
        if (*record).count == 0 {
            let shape = (*record).shape;
            let fields = (*shape).fields as usize;
            let mut slot = fields_of(record);
            for index in 0..fields {
                let (kind, _) = field_of(shape, index);
                release_field(kind, slot);
                slot = slot.add(words_of(kind));
            }
            if cfg!(debug_assertions) {
                // Left readable as freed, as a String is, so a second
                // release is caught above.
                LIVE_STRUCTS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
            std::alloc::dealloc(record.cast(), struct_layout(struct_words(shape)));
        }
    }
}

/// Whether a struct that may not be there is of this shape (#572): one
/// that is not there is of none.
///
/// # Safety
/// `record` is null or points at a live struct, and `shape` at a shape
/// the compiler wrote.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_struct_is(
    record: *mut StructBlock,
    shape: *const StructShape,
) -> bool {
    unsafe { !record.is_null() && std::ptr::eq((*record).shape, shape) }
}

/// Where the field of this name is in a struct that is one of several
/// (#572): the compiled program knows the field's type, and the struct
/// itself says where it keeps it. A struct with no such field refuses in
/// the seed's words.
///
/// # Safety
/// `record` points at a live struct whose fields are what its shape says,
/// and `name` at a String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_struct_slot(
    record: *mut StructBlock,
    name: *const StringBlock,
) -> *mut i64 {
    unsafe {
        let shape = (*record).shape;
        let wanted = bytes_of(name);
        let mut slot = fields_of(record);
        for index in 0..(*shape).fields as usize {
            let (kind, field) = field_of(shape, index);
            if bytes_of(field) == wanted {
                return slot;
            }
            slot = slot.add(words_of(kind));
        }
        let struct_name = std::str::from_utf8_unchecked(bytes_of((*shape).name));
        let article = if struct_name.starts_with(['A', 'E', 'I', 'O', 'U']) {
            "an"
        } else {
            "a"
        };
        refuse_program(&format!(
            "{} is {article} {struct_name}, which has no method '{}'",
            struct_source(record),
            std::str::from_utf8_unchecked(wanted)
        ))
    }
}

/// A struct's source form: its name, and each field by name with its
/// value written as its kind says.
///
/// # Safety
/// `record` points at a live struct whose fields are what its shape says.
unsafe fn struct_source(record: *mut StructBlock) -> String {
    unsafe {
        let shape = (*record).shape;
        let mut written = Vec::with_capacity((*shape).fields as usize);
        let mut slot = fields_of(record);
        for index in 0..(*shape).fields as usize {
            let (kind, name) = field_of(shape, index);
            written.push(format!(
                "{}: {}",
                std::str::from_utf8_unchecked(bytes_of(name)),
                field_source(kind, slot)
            ));
            slot = slot.add(words_of(kind));
        }
        format!(
            "{}({})",
            std::str::from_utf8_unchecked(bytes_of((*shape).name)),
            written.join(", ")
        )
    }
}

/// A struct written out, which is its source form whether `puts`,
/// `inspect`, or interpolation asks: a new String, held once. The
/// absence of a struct is answered by absence.
///
/// # Safety
/// `record` is null or points at a live struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_struct_to_string(record: *mut StructBlock) -> *mut StringBlock {
    if record.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { string_from(&[struct_source(record).as_bytes()]) }
}

/// What the process was started with, after its own name: kept from the
/// entry function's first act for `System.arguments` to answer.
static ARGUMENTS: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();

/// The entry function's first act (#534): the command line, as the
/// operating system handed it over, kept for the program to ask for.
///
/// # Safety
/// `values` points at `count` strings that end in a zero byte, as `main`
/// is handed them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_start(count: i32, values: *const *const libc::c_char) {
    let mut arguments = Vec::new();
    for index in 1..count.max(0) as usize {
        let value = unsafe { std::ffi::CStr::from_ptr(values.add(index).read()) };
        arguments.push(value.to_bytes().to_vec());
    }
    let _ = ARGUMENTS.set(arguments);
}

/// `System.arguments` (ADR 0068): what the process was started with, as
/// a new array of new Strings, held once.
#[unsafe(no_mangle)]
pub extern "C" fn portland_system_arguments() -> *mut ArrayBlock {
    let arguments = ARGUMENTS.get().map(Vec::as_slice).unwrap_or(&[]);
    let mut array = array_with(STRINGS, arguments.len());
    for argument in arguments {
        array = unsafe { portland_array_append(array, string_from(&[argument]) as i64, STRINGS) };
    }
    array
}

/// `System.read_file(path)` (ADR 0068): the file's text as a new String,
/// held once, or nothing where it cannot be read or is not UTF-8. One
/// crossing: open, read to the end, close.
///
/// # Safety
/// `path` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_system_read_file(path: *const StringBlock) -> *mut StringBlock {
    let path = unsafe { std::str::from_utf8_unchecked(bytes_of(path)) };
    match std::fs::read_to_string(path) {
        Ok(content) => string_from(&[content.as_bytes()]),
        Err(_) => std::ptr::null_mut(),
    }
}

/// What a refusal calls a value: its source form, or `nil` where there
/// is none.
///
/// # Safety
/// `text` is null or points at a live String, whose bytes are UTF-8.
unsafe fn named_in_refusal(text: *const StringBlock) -> String {
    if text.is_null() {
        return "nil".to_string();
    }
    unsafe { std::str::from_utf8_unchecked(bytes_of(text)).to_string() }
}

/// `panic(message)`: the program stops with what it was told to say, as
/// the seed stops.
///
/// # Safety
/// `message` points at a live String, whose bytes are UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_panic(message: *const StringBlock) -> ! {
    let message = unsafe { named_in_refusal(message) };
    refuse_program(&message);
}

/// A method asked of nil (ADR 0096): a maybe was used where its value
/// was needed and had none. The seed's sentence.
///
/// # Safety
/// `name` points at a live String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_refuse_nil_method(name: *const StringBlock) -> ! {
    let name = unsafe { named_in_refusal(name) };
    refuse_program(&format!(
        "nil has no method '{name}' — handle the nil case first"
    ));
}

/// `puts` handed nil. The seed's sentence.
#[unsafe(no_mangle)]
pub extern "C" fn portland_refuse_nil_puts() -> ! {
    refuse_program("puts got nil — handle the nil case first (p renders nil for debugging)");
}

/// An operator handed nil on one side or both. Each side arrives in its
/// source form, and absent where it is the nil. The seed's sentence.
///
/// # Safety
/// `operator` points at a live String; `left` and `right` are each null
/// or point at one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_refuse_nil_operand(
    operator: *const StringBlock,
    left: *const StringBlock,
    right: *const StringBlock,
) -> ! {
    let (operator, left, right) = unsafe {
        (
            named_in_refusal(operator),
            named_in_refusal(left),
            named_in_refusal(right),
        )
    };
    refuse_program(&format!("cannot apply '{operator}' to {left} and {right}"));
}

/// A refusal of the runtime's own: what was written so far, then the
/// sentence on standard error, and a status no Portland program exits with.
fn refuse(sentence: &str) -> ! {
    unsafe {
        libc::fflush(__stdoutp);
    }
    eprintln!("{sentence}");
    std::process::exit(70);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tally of live Strings is one number for the whole process, and
    /// tests run side by side, so each test that makes a String holds this
    /// while it counts.
    static COUNTING: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn text_of(text: *const StringBlock) -> String {
        String::from_utf8(unsafe { bytes_of(text) }.to_vec()).unwrap()
    }

    #[test]
    fn a_made_string_is_held_once_and_freed_by_its_last_release() {
        let _counting = COUNTING.lock().unwrap();
        let before = live_strings();
        let number = portland_string_from_integer(-42);
        assert_eq!(text_of(number), "-42");
        let joined = unsafe { portland_string_concat(number, number) };
        assert_eq!(text_of(joined), "-42-42");
        assert_eq!(live_strings(), before + 2);
        unsafe {
            portland_retain(joined);
            portland_release(joined);
            assert_eq!(live_strings(), before + 2, "one holder is left");
            portland_release(joined);
            portland_release(number);
        }
        assert_eq!(live_strings(), before);
    }

    fn words_of(array: *mut ArrayBlock) -> Vec<i64> {
        unsafe { std::slice::from_raw_parts(elements_of(array), (*array).length as usize).to_vec() }
    }

    #[test]
    fn an_array_held_once_grows_where_it_is_and_one_held_elsewhere_is_copied() {
        let _counting = COUNTING.lock().unwrap();
        let before = live_arrays();
        let mut box_of = portland_array_new(INTEGERS, 0);
        unsafe {
            for number in 1..=9 {
                box_of = portland_array_append(box_of, number, INTEGERS);
            }
            assert_eq!(words_of(box_of), (1..=9).collect::<Vec<i64>>());
            assert_eq!(live_arrays(), before + 1, "growing makes no second array");
            // A second name holds it, so an append answers a copy and the
            // first name's array is as it was.
            portland_retain(box_of.cast());
            let other = box_of;
            let grown = portland_array_append(box_of, 10, INTEGERS);
            assert_eq!(words_of(other).len(), 9);
            assert_eq!(words_of(grown).len(), 10);
            assert_eq!(live_arrays(), before + 2);
            portland_array_release(other);
            portland_array_release(grown);
        }
        assert_eq!(live_arrays(), before);
    }

    #[test]
    fn an_array_holds_each_counted_element_once() {
        let _counting = COUNTING.lock().unwrap();
        let (strings, arrays) = (live_strings(), live_arrays());
        unsafe {
            let mut names = portland_array_new(STRINGS, 0);
            for number in 0..6 {
                names = portland_array_append(
                    names,
                    portland_string_from_integer(number) as i64,
                    STRINGS,
                );
            }
            assert_eq!(live_strings(), strings + 6);
            // A copy holds every element too, so freeing the first array
            // frees none of them.
            portland_retain(names.cast());
            let grown =
                portland_array_append(names, portland_string_from_integer(6) as i64, STRINGS);
            portland_array_release(names);
            assert_eq!(live_strings(), strings + 7);
            // An array of arrays frees what it holds, all the way down.
            let rows = portland_array_append(portland_array_new(ARRAYS, 1), grown as i64, ARRAYS);
            assert_eq!(live_arrays(), arrays + 2);
            portland_array_release(rows);
        }
        assert_eq!(live_strings(), strings);
        assert_eq!(live_arrays(), arrays);
    }

    #[test]
    fn an_array_is_written_out_in_its_source_form() {
        let _counting = COUNTING.lock().unwrap();
        unsafe {
            let mut names = portland_array_new(STRINGS, 0);
            for written in [
                "ada",
                "say \"hi\"",
                "tab\there",
                "#{x}",
                "bell\u{7}",
                "\u{2028}",
            ] {
                names = portland_array_append(
                    names,
                    string_from(&[written.as_bytes()]) as i64,
                    STRINGS,
                );
            }
            let source = portland_array_to_string(names);
            assert_eq!(
                text_of(source),
                "[\"ada\", \"say \\\"hi\\\"\", \"tab\\there\", \"\\#{x}\", \"bell\\a\", \"\\u2028\"]"
            );
            portland_release(source);
            let mut flags = portland_array_new(BOOLEANS, 0);
            flags = portland_array_append(flags, 1, BOOLEANS);
            flags = portland_array_append(flags, 0, BOOLEANS);
            let rows = portland_array_append(portland_array_new(ARRAYS, 1), flags as i64, ARRAYS);
            let source = portland_array_to_string(rows);
            assert_eq!(text_of(source), "[[true, false]]");
            portland_release(source);
            portland_array_release(rows);
            portland_array_release(names);
            let truth = portland_string_from_boolean(true);
            assert_eq!(text_of(truth), "true");
            portland_release(truth);
        }
    }

    #[test]
    fn a_struct_holds_its_fields_and_writes_itself_out() {
        #[repr(C)]
        struct Shape {
            header: StructShape,
            fields: [i64; 4],
        }
        let _counting = COUNTING.lock().unwrap();
        let (strings, structs) = (live_strings(), live_structs());
        unsafe {
            let name = string_from(&[b"Named"]);
            let label = string_from(&[b"label"]);
            let rank = string_from(&[b"rank"]);
            let shape = Shape {
                header: StructShape { name, fields: 2 },
                fields: [STRINGS, label as i64, INTEGERS, rank as i64],
            };
            let pointer = std::ptr::addr_of!(shape).cast::<StructShape>();
            let record = portland_struct_new(pointer);
            fields_of(record).write(string_from(&[b"home"]) as i64);
            fields_of(record).add(1).write(7);
            assert_eq!(live_structs(), structs + 1);
            let source = portland_struct_to_string(record);
            assert_eq!(text_of(source), "Named(label: \"home\", rank: 7)");
            portland_release(source);
            // An array of structs holds each, and frees what they hold.
            let held =
                portland_array_append(portland_array_new(STRUCTS, 1), record as i64, STRUCTS);
            let listed = portland_array_to_string(held);
            assert_eq!(text_of(listed), "[Named(label: \"home\", rank: 7)]");
            portland_release(listed);
            portland_array_release(held);
            assert_eq!(live_structs(), structs);
            portland_release(name);
            portland_release(label);
            portland_release(rank);
        }
        assert_eq!(live_strings(), strings);
    }

    #[test]
    fn the_empty_literal_is_never_written_to() {
        let _counting = COUNTING.lock().unwrap();
        let before = live_arrays();
        let mut literal = ArrayBlock {
            count: LITERAL,
            length: 0,
            capacity: 0,
            kind: INTEGERS,
        };
        let empty = std::ptr::addr_of_mut!(literal);
        unsafe {
            let first = portland_array_append(empty, 7, INTEGERS);
            assert_eq!(words_of(first), vec![7]);
            assert_eq!((*empty).count, LITERAL);
            assert_eq!((*empty).length, 0);
            portland_array_release(empty);
            portland_array_release(first);
            portland_array_release(std::ptr::null_mut());
        }
        assert_eq!(live_arrays(), before);
    }

    #[test]
    fn division_floors_and_the_remainder_takes_the_divisors_sign() {
        assert_eq!(portland_integer_divide(7, 2), 3);
        assert_eq!(portland_integer_divide(-7, 2), -4);
        assert_eq!(portland_integer_divide(7, -2), -4);
        assert_eq!(portland_integer_divide(-7, -2), 3);
        assert_eq!(portland_integer_divide(-6, 2), -3);
        assert_eq!(portland_integer_modulo(7, 3), 1);
        assert_eq!(portland_integer_modulo(-7, 3), 2);
        assert_eq!(portland_integer_modulo(7, -3), -2);
        assert_eq!(portland_integer_modulo(-7, -3), -1);
        assert_eq!(portland_integer_modulo(i64::MIN, -1), 0);
        assert_eq!(portland_integer_negate(5), -5);
        assert_eq!(portland_integer_negate(i64::MAX), i64::MIN + 1);
    }

    #[test]
    fn a_strings_length_counts_grapheme_clusters() {
        let _counting = COUNTING.lock().unwrap();
        let cases = [
            ("", 0),
            ("abc", 3),
            ("h\u{e9}llo", 5),
            ("e\u{301}x", 2),
            ("\u{1F1FA}\u{1F1F8}", 1),
        ];
        for (written, expected) in cases {
            let text = string_from(&[written.as_bytes()]);
            assert_eq!(
                unsafe { portland_string_length(text) },
                expected,
                "{written:?}"
            );
            unsafe { portland_release(text) };
        }
        assert_eq!(portland_integer_abs(-7), 7);
        assert_eq!(portland_integer_succ(-7), -6);
        assert_eq!(portland_integer_pred(-7), -8);
    }

    #[test]
    fn equality_is_canonical() {
        let _counting = COUNTING.lock().unwrap();
        let composed = string_from(&["caf\u{e9}".as_bytes()]);
        let decomposed = string_from(&["cafe\u{301}".as_bytes()]);
        let other = string_from(&["cafe".as_bytes()]);
        unsafe {
            assert!(portland_string_equal(composed, composed));
            assert!(portland_string_equal(composed, decomposed));
            assert!(portland_string_equal(decomposed, composed));
            assert!(!portland_string_equal(composed, other));
            portland_release(composed);
            portland_release(decomposed);
            portland_release(other);
        }
    }

    #[test]
    fn a_literal_and_an_unbound_local_are_not_counted() {
        #[repr(C)]
        struct Literal {
            header: StringBlock,
            bytes: [u8; 2],
        }
        let mut literal = Literal {
            header: StringBlock {
                count: LITERAL,
                length: 2,
            },
            bytes: *b"hi",
        };
        let pointer = std::ptr::addr_of_mut!(literal).cast::<StringBlock>();
        unsafe {
            portland_retain(pointer);
            portland_release(pointer);
            portland_release(pointer);
            assert_eq!((*pointer).count, LITERAL);
            portland_retain(std::ptr::null_mut());
            portland_release(std::ptr::null_mut());
        }
        assert_eq!(text_of(pointer), "hi");
    }
}

mod library;
