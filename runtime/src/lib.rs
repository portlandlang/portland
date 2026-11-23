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
