// Copyright 2018-2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

#[ink::event]
pub struct EventExternal {
    f1: bool,
    f2: u32,
}

#[ink::contract]
mod contract {
    #[ink(storage)]
    pub struct Contract {}

    #[ink(event)]
    pub struct EventInline {
        f3: bool,
        f4: u32,
    }

    impl Contract {
        #[ink(constructor)]
        pub fn new(_x: u8) -> Self {
            Self {}
        }
    }

    impl Contract {
        #[ink(message)]
        pub fn get_value(&self) -> u32 {
            42
        }
    }
}

#[cfg(test)]
mod tests {
    fn generate_metadata() -> ink_metadata::InkProject {
        extern "Rust" {
            fn __ink_generate_metadata() -> ink_metadata::InkProject;
        }

        unsafe { __ink_generate_metadata() }
    }

    #[test]
    fn collects_all_events() {
        let metadata = generate_metadata();

        assert_eq!(metadata.spec().events().len(), 2);
        assert!(metadata
            .spec()
            .events()
            .iter()
            .any(|e| e.label() == "EventExternal"));
        assert!(metadata
            .spec()
            .events()
            .iter()
            .any(|e| e.label() == "EventInline"));
    }
}
