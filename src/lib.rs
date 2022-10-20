/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![cfg_attr(feature = "fatal-warnings", deny(warnings))]
#![deny(clippy::correctness)]
#![warn(clippy::pedantic)]
// Note: If you change this remember to update `README.md`.  To do so run `./tools/update-readme.sh`.
//! WIP

pub fn foo() {
    println!("here")
}

#[cfg(test)]
mod test {
    #[test]
    fn test_foo() {
        super::foo();
    }
}
