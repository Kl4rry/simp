// Copied from lexical-sort
/*
Copyright (c) 2020 Ludwig Stecher and contributors

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
*/

use std::cmp::Ordering;

macro_rules! cmp_ascii_digits {
    (first_digits($lhs:ident, $rhs:ident), iterators($iter1:ident, $iter2:ident)) => {
        let mut n1 = ascii_to_u64($lhs);
        let mut n2 = ascii_to_u64($rhs);
        loop {
            match (
                $iter1.peek().copied().filter(|c| c.is_ascii_digit()),
                $iter2.peek().copied().filter(|c| c.is_ascii_digit()),
            ) {
                (Some(lhs), Some(rhs)) => {
                    // This results in wrong sorting for very big numbers
                    // but it will never crash
                    n1 = n1.wrapping_mul(10).wrapping_add(ascii_to_u64(lhs));
                    n2 = n2.wrapping_mul(10).wrapping_add(ascii_to_u64(rhs));
                    let _ = $iter1.next();
                    let _ = $iter2.next();
                }
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => {
                    if n1 != n2 {
                        return n1.cmp(&n2);
                    } else {
                        break;
                    }
                }
            }
        }
    };
}

#[inline]
fn ascii_to_u64(c: char) -> u64 {
    (c as u64) - (b'0' as u64)
}

pub fn natural_cmp(s1: &str, s2: &str) -> Ordering {
    let mut iter1 = s1.chars().peekable();
    let mut iter2 = s2.chars().peekable();

    loop {
        match (iter1.next(), iter2.next()) {
            (Some(lhs), Some(rhs)) => {
                if lhs.is_ascii_digit() && rhs.is_ascii_digit() {
                    cmp_ascii_digits!(first_digits(lhs, rhs), iterators(iter1, iter2));
                } else if lhs != rhs {
                    return lhs.cmp(&rhs);
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }
    }
}
