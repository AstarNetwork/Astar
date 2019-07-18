use super::*;
use ink_core::storage::{
    alloc::{AllocateUsing, Initialize},
    Flush,
};

#[macro_use]
use super::state;

/// Define contract state with less boilerplate code.
#[macro_export]
macro_rules! state {
	(
		$( #[$state_meta:meta] )*
		$vis:vis struct $state_name:ident
		{
			$(
				$( #[$field_meta:meta] )*
				$field_name:ident : $field_ty:ty ,
			)*
		}
	) => {
		$( #[$state_meta] )*
		$vis struct $state_name<T: Member + Codec> {
			$(
				$( #[$field_meta] )*
				$field_name : $field_ty
			),*
		}

		impl<T: Member + Codec> ink_core::storage::Flush for $state_name<T> {
			fn flush(&mut self) {
				$(
					self.$field_name.flush()
				);*
			}
		}

		impl<T: Member + Codec> ink_core::storage::alloc::AllocateUsing for $state_name<T> {
			unsafe fn allocate_using<A>(alloc: &mut A) -> Self
			where
				A: ink_core::storage::alloc::Allocate,
			{
				use ink_core::storage::alloc::AllocateUsing;
				Self {
					$(
						$field_name : AllocateUsing::allocate_using(alloc)
					),*
				}
			}
		}

        impl<T: Member + Codec> ink_core::storage::alloc::Initialize for $state_name<T> {
            type Args = ();

            #[inline(always)]
            fn default_value() -> Option<Self::Args> {
                // With this we can also default initialize storage state structs.
                Some(())
            }

            fn initialize(&mut self, args: Self::Args) {
                $(
                    self.$field_name.try_default_initialize();
                )*
            }
        }

		impl<T: Member + Codec> ink_model::ContractState for $state_name<T> {
			const NAME: &'static str = stringify!($state_name);
		}
	};
	(
		$( #[$state_meta:meta] )*
		$vis:vis struct $state_name:ident {
			$(
				$( #[$field_meta:meta] )*
				$field_name:ident : $field_ty:ty
			),*
		}
	) => {
		$crate::state! {
			$vis struct $state_name {
				$(
					$( #[$field_meta] )*
					$field_name : $field_ty ,
				)*
			}
		}
	};
}
