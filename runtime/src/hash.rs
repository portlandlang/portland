//! A Hash as a native program holds it (#607): String keys, values of
//! one kind, and the order the keys went in, which a walk follows as the
//! seed's does.
//!
//! It is two arrays side by side, the keys and the values, so a value is
//! held, released, and written out by what is already there for an
//! array's elements. A key is found by reading the keys in turn: the
//! plain way first, and a faster one when a measurement asks for it.
//!
//! Every function here borrows what it is handed, as the library's do,
//! except where it says it takes the caller's hold.

use super::*;

/// The header a Hash points at. Its count comes first, as every counted
/// block's does, so `portland_retain` holds it as it holds the rest.
#[repr(C)]
pub struct HashBlock {
    /// How many names hold this Hash.
    count: i64,
    /// The keys, Strings, in the order they went in.
    keys: *mut ArrayBlock,
    /// The value for each key, at the key's place.
    values: *mut ArrayBlock,
}

/// How many Hashes made at run time are alive, kept by the debug build.
static LIVE_HASHES: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// The Hashes made at run time and not yet freed: zero in a release
/// build, which does not keep the tally.
pub fn live_hashes() -> i64 {
    LIVE_HASHES.load(std::sync::atomic::Ordering::Relaxed)
}

/// The place of a key among a Hash's keys, if it is there.
///
/// # Safety
/// `keys` points at a live array of Strings, and `key` at a live String.
unsafe fn place_of(keys: *mut ArrayBlock, key: *const StringBlock) -> Option<usize> {
    unsafe {
        let sought = bytes_of(key);
        (0..(*keys).length as usize).find(|&index| {
            bytes_of(elements_of(keys).add(index).read() as *const StringBlock) == sought
        })
    }
}

/// `{}`: a new Hash with no pairs, held once, whose values will be of
/// this kind.
#[unsafe(no_mangle)]
pub extern "C" fn portland_hash_empty(kind: i64) -> *mut HashBlock {
    Pairs::with_room(kind, 0).into_hash()
}

/// A new Hash from its keys and values, pair by pair in order, held once.
/// A key written twice keeps its first place and its last value, as
/// Ruby's literal does. Takes the caller's hold on both arrays.
///
/// # Safety
/// `keys` points at a live array of Strings, and `values` at a live array
/// of the same length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_hash_new(
    keys: *mut ArrayBlock,
    values: *mut ArrayBlock,
) -> *mut HashBlock {
    unsafe {
        let mut kept = Pairs::with_room((*values).kind, (*keys).length as usize);
        kept.put_all(keys, values);
        portland_array_release(keys);
        portland_array_release(values);
        kept.into_hash()
    }
}

/// `hash.merge(other)`: a new Hash, held once, with the receiver's pairs
/// and then the other's, the other's value where both have a key, as the
/// seed's merge answers. Neither Hash changes.
///
/// # Safety
/// `hash` and `other` point at live Hashes whose values are of one kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_hash_merge(
    hash: *const HashBlock,
    other: *const HashBlock,
) -> *mut HashBlock {
    unsafe {
        let room = (*(*hash).keys).length + (*(*other).keys).length;
        let mut kept = Pairs::with_room((*(*hash).values).kind, room as usize);
        kept.put_all((*hash).keys, (*hash).values);
        kept.put_all((*other).keys, (*other).values);
        kept.into_hash()
    }
}

/// `hash.key?(key)`, and its other names: whether the key is there.
///
/// # Safety
/// `hash` points at a live Hash, and `key` at a live String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_hash_key(
    hash: *const HashBlock,
    key: *const StringBlock,
) -> bool {
    unsafe { place_of((*hash).keys, key).is_some() }
}

/// The keys and values of a Hash being made, each array held once.
struct Pairs {
    keys: *mut ArrayBlock,
    values: *mut ArrayBlock,
    kind: i64,
}

impl Pairs {
    /// None yet, with room for this many.
    fn with_room(kind: i64, room: usize) -> Pairs {
        Pairs {
            keys: array_with(STRINGS, room.max(1)),
            values: array_with(kind, room.max(1)),
            kind,
        }
    }

    /// One pair put in: where the key is there already, its value is
    /// replaced in its place; otherwise the pair goes at the end. Each
    /// kept word gains a holder, and a replaced value loses one.
    ///
    /// # Safety
    /// `key` is a live String, and `value` a word of the pairs' kind.
    unsafe fn put(&mut self, key: i64, value: i64) {
        unsafe {
            hold_word(self.kind, value, true);
            match place_of(self.keys, key as *const StringBlock) {
                Some(place) => {
                    let slot = elements_of(self.values).add(place);
                    hold_word(self.kind, slot.read(), false);
                    slot.write(value);
                }
                None => {
                    hold_word(STRINGS, key, true);
                    self.keys = portland_array_append(self.keys, key, STRINGS);
                    self.values = portland_array_append(self.values, value, self.kind);
                }
            }
        }
    }

    /// Every pair of these two arrays put in, in order. Borrows both.
    ///
    /// # Safety
    /// `keys` is a live array of Strings, and `values` a live array of the
    /// pairs' kind and the same length.
    unsafe fn put_all(&mut self, keys: *mut ArrayBlock, values: *mut ArrayBlock) {
        unsafe {
            for index in 0..(*keys).length as usize {
                self.put(
                    elements_of(keys).add(index).read(),
                    elements_of(values).add(index).read(),
                );
            }
        }
    }

    /// The Hash these pairs make, which takes both holds.
    fn into_hash(self) -> *mut HashBlock {
        hash_of(self.keys, self.values)
    }
}

/// A Hash around these two arrays, held once, taking the hold on each.
fn hash_of(keys: *mut ArrayBlock, values: *mut ArrayBlock) -> *mut HashBlock {
    let layout = std::alloc::Layout::new::<HashBlock>();
    unsafe {
        let block = std::alloc::alloc(layout).cast::<HashBlock>();
        if block.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        block.write(HashBlock {
            count: 1,
            keys,
            values,
        });
        if cfg!(debug_assertions) {
            LIVE_HASHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        block
    }
}

/// `hash[key]`: the value, held once, or nothing where the key is not
/// there — the least Integer for a Hash of Integers, and null for one of
/// counted values, as the library answers a maybe.
///
/// # Safety
/// `hash` points at a live Hash, and `key` at a live String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_hash_get(hash: *const HashBlock, key: *const StringBlock) -> i64 {
    unsafe {
        let values = (*hash).values;
        let kind = (*values).kind;
        match place_of((*hash).keys, key) {
            Some(place) => {
                let value = elements_of(values).add(place).read();
                hold_word(kind, value, true);
                value
            }
            None if kind == INTEGERS => i64::MIN,
            None => 0,
        }
    }
}

/// `hash.length`: how many keys it holds.
///
/// # Safety
/// `hash` points at a live Hash.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_hash_length(hash: *const HashBlock) -> i64 {
    unsafe { (*(*hash).keys).length }
}

/// One name fewer holds this Hash, and the last one frees it, with its
/// two arrays.
///
/// # Safety
/// `hash` is null or points at a Hash that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn portland_hash_release(hash: *mut HashBlock) {
    unsafe {
        if hash.is_null() {
            return;
        }
        if cfg!(debug_assertions) && (*hash).count <= 0 {
            refuse(
                "a Hash was released more times than it was held — the native build counted wrong (#607)",
            );
        }
        (*hash).count -= 1;
        if (*hash).count == 0 {
            portland_array_release((*hash).keys);
            portland_array_release((*hash).values);
            if cfg!(debug_assertions) {
                // Left readable as freed, as an array is, so a second
                // release is caught above.
                LIVE_HASHES.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
            std::alloc::dealloc(hash.cast(), std::alloc::Layout::new::<HashBlock>());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(texts: &[&str]) -> *mut ArrayBlock {
        let mut array = array_with(STRINGS, texts.len());
        for text in texts {
            array = unsafe {
                portland_array_append(array, string_from(&[text.as_bytes()]) as i64, STRINGS)
            };
        }
        array
    }

    fn integers(numbers: &[i64]) -> *mut ArrayBlock {
        let mut array = array_with(INTEGERS, numbers.len());
        for number in numbers {
            array = unsafe { portland_array_append(array, *number, INTEGERS) };
        }
        array
    }

    #[test]
    fn a_key_written_twice_keeps_its_first_place_and_its_last_value() {
        let _counting = crate::tests::COUNTING.lock().unwrap();
        unsafe {
            let hash = portland_hash_new(strings(&["fern", "moss", "fern"]), integers(&[1, 2, 3]));
            assert_eq!(portland_hash_length(hash), 2);
            let fern = string_from(&[b"fern"]);
            assert_eq!(portland_hash_get(hash, fern), 3);
            let oak = string_from(&[b"oak"]);
            assert_eq!(portland_hash_get(hash, oak), i64::MIN);
            portland_release(fern);
            portland_release(oak);
            portland_hash_release(hash);
        }
    }
}
