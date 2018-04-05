// Copyright 2018 Eduardo Sánchez Muñoz
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std;
use std::cell::Cell;

use Rob;

// tests might run in multiple threads
thread_local! {
    static DROP_COUNT: Cell<usize> = Cell::new(0);
}

#[derive(Clone)]
struct TestObj(i32);

impl Drop for TestObj {
    fn drop(&mut self) {
        DROP_COUNT.with(|cnt| cnt.set(cnt.get() + 1));
    }
}

#[test]
fn test_from_value() {
    DROP_COUNT.with(|cnt| cnt.set(0));
    let x = Rob::from_value(TestObj(123));
    assert!(Rob::is_owned(&x));
    assert_eq!(x.0, 123);
    std::mem::drop(x);
    assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 1);
}

#[test]
fn test_from_box() {
    DROP_COUNT.with(|cnt| cnt.set(0));
    let x = Rob::from_box(Box::new(TestObj(123)));
    assert!(Rob::is_owned(&x));
    assert_eq!(x.0, 123);
    std::mem::drop(x);
    assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 1);
}

#[test]
fn test_from_ref() {
    DROP_COUNT.with(|cnt| cnt.set(0));
    let obj = TestObj(123);
    {
        let x = Rob::from_ref(&obj);
        assert!(!Rob::is_owned(&x));
        assert_eq!(x.0, 123);
        std::mem::drop(x);
        assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 0);
    }
    std::mem::drop(obj);
    assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 1);
}

#[test]
fn test_owned_into_box() {
    DROP_COUNT.with(|cnt| cnt.set(0));
    let x = Rob::from_value(TestObj(123));
    let b = Rob::into_box(x);
    assert_eq!(b.0, 123);
    std::mem::drop(b);
    assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 1);
}

#[test]
fn test_borrowed_into_box() {
    DROP_COUNT.with(|cnt| cnt.set(0));
    let obj = TestObj(123);
    {
        let x = Rob::from_ref(&obj);
        let b = Rob::into_box(x);
        assert_eq!(b.0, 123);
        std::mem::drop(b);
        assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 1);
    }
    std::mem::drop(obj);
    assert_eq!(DROP_COUNT.with(|cnt| cnt.get()), 2);
}
