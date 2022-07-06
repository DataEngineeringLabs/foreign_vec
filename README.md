This library offers [`ForeignVec`], a zero-cost abstraction to store either [`Vec<T>`]
or an immutable region aligned with `T` allocated by an external allocator.

The primary use-case of this library is when you have an in-memory representation
in both Rust and other languages and you have a specification to share
(immutable) vectors across language boundaries at zero cost, via FFI, while
still allowing mutable access to the region when it is allocated by Rust.
In other words:

Given:
* an in-memory format
* an FFI spec to share immutable memory regions at zero cost

Then, [`ForeignVec`] offers an interface to

* allow zero-cost immutable access via `core::ops::Deref<T>` to `Vec<T>` or
  the foreign vector
* allow access to `&mut Vec<T>` (when it is allocated by Rust)

The complete tests:

```rust
use foreign_vec::ForeignVec;

// say that we have a foreign struct allocated by an external allocator (e.g. C++)
// owning an immutable memory region
struct Foreign {
    ptr: *const i32,
    length: usize,
    capacity: usize,
}

// whose drop calls an external function that deallocates the region
impl Drop for Foreign {
    fn drop(&mut self) {
        // mocking an external deallocation
        unsafe { Vec::from_raw_parts(self.ptr as *mut i32, self.length, self.capacity) };
    }
}

// The type that we use on the library uses `foreign_vec`
type MyForeignVec<T> = ForeignVec<Foreign, T>;

#[test]
fn test_vec() {
    // we can use it with `Vec`:
    let expected: &[i32] = &[1, 2];

    // when we have a vector, we can use `.into()`
    let vec = expected.to_vec();
    let mut vec: MyForeignVec<i32> = vec.into();

    // deref works as expected
    assert_eq!(&*vec, expected);

    // debug works as expected
    assert_eq!(format!("{:?}", vec), "[1, 2]");

    // you can retrieve a mut vec (since it is allocated by Rust)
    assert_eq!(vec.get_vec(), Some(&mut vec![1, 2]));

    // this calls `Vec::drop`, as usual
    drop(vec)
}

// this is just `Vec::into_raw_parts`, which is still unstable
fn into_raw_parts<T>(vec: Vec<T>) -> (*mut T, usize, usize) {
    let r = (vec.as_ptr() as *mut T, vec.len(), vec.capacity());
    std::mem::forget(vec);
    r
}

#[test]
fn test_foreign() {
    // and on an externally allocated pointer (here from Rust, but a foreign call would do the same)
    let expected: &[i32] = &[1, 2];

    let a = expected.to_vec();
    let (ptr, length, capacity) = into_raw_parts(a);
    let a = Foreign {
        ptr,
        length,
        capacity,
    };

    let mut vec = unsafe { MyForeignVec::from_owned(a.ptr, a.length, a) };
    assert_eq!(&*vec, expected);
    assert_eq!(vec.get_vec(), None);

    // this calls `Foreign::drop`, which calls the foreign function
    drop(vec);
}
```
