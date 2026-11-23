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
//! workspace, never source.

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

/// One more name holds this String. A literal is not counted, and neither
/// is the absence of a String: a local not yet bound.
///
/// # Safety
/// `text` is null or points at a live String.
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
/// every String made at run time must have been freed. One that was not
/// means the compiler missed a release.
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

    fn text_of(text: *const StringBlock) -> String {
        String::from_utf8(unsafe { bytes_of(text) }.to_vec()).unwrap()
    }

    #[test]
    fn a_made_string_is_held_once_and_freed_by_its_last_release() {
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
