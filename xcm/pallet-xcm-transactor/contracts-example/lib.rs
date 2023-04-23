#![cfg_attr(not(feature = "std"), no_std)]

mod sdk;
pub use sdk::*;

#[ink::contract]
mod contracts {
    use super::*;
    use ink::env::DefaultEnvironment;
    use xcm::{latest::Weight, prelude::*};
    use xcm_ce_types::Error as XcmCEError;

    #[ink(storage)]
    pub struct Contracts {
        value: bool,
    }

    impl Contracts {
        #[ink(constructor)]
        pub fn new(init_value: bool) -> Self {
            Self { value: init_value }
        }

        #[ink(constructor, selector = 0xC0DECFFF)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        #[ink(message, selector = 0xC0DECAFE)]
        pub fn xcm_flip(
            &mut self,
            _query_id: QueryId,
            _responder: MultiLocation,
            _response: Response,
        ) {
            self.value = !self.value;
        }

        #[ink(message, selector = 0xC0DEC000)]
        pub fn test(&mut self, xcm: VersionedXcm<()>) -> Result<Weight, XcmCEError> {
            let a = XcmExtension::<DefaultEnvironment>::prepare_execute(xcm);
            ink::env::debug_println!("{a:?}");
            a
        }

        #[ink(message, selector = 0xC0DECEEE)]
        pub fn get(&self) -> bool {
            self.value
        }
    }
}
