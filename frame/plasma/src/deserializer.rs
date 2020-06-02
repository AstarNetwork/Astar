use super::*;
use sp_std::marker::PhantomData;

pub struct Deserializer<T: Trait>(PhantomData<T>);

impl<T: Trait> Deserializer<T> {
    /// @dev deserialize property to Exit instance
    pub fn deserialize_exit(exit: &PropertyOf<T>) -> DispatchResultT<ExitOf<T>> {
        let state_update_property: PropertyOf<T> =
            Decode::decode(&mut &exit.inputs[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        let inclusion_proof: InclusionProofOf<T> =
            Decode::decode(&mut &exit.inputs[1][..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        Ok(ExitOf::<T> {
            state_update: Self::deserialize_state_update(&state_update_property)?,
            inclusion_proof,
        })
    }

    /// @dev deserialize property to Exit_deposit instance
    pub fn deserialize_exit_deposit(exit: &PropertyOf<T>) -> DispatchResultT<ExitDepositOf<T>> {
        let state_update_property: PropertyOf<T> =
            Decode::decode(&mut &exit.inputs[0][..]).map_err(|_| Error::<T>::MustBeDecodable)?;
        let checkpoint_property: PropertyOf<T> =
            Decode::decode(&mut &exit.inputs[1][..]).map_err(|_| Error::<T>::MustBeDecodable)?;

        Ok(ExitDeposit {
            state_update: Self::deserialize_state_update(&state_update_property)?,
            checkpoint: Self::deserialize_checkpoint(&checkpoint_property)?,
        })
    }

    /// @dev deserialize property to State_update instance
    pub fn deserialize_state_update(
        state_update: &PropertyOf<T>,
    ) -> DispatchResultT<StateUpdateOf<T>> {
        let deposit_address: T::AccountId = Decode::decode(&mut &state_update.inputs[0][..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;
        let range: RangeOf<T> = Decode::decode(&mut &state_update.inputs[1][..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;

        let block_number: T::BlockNumber = Decode::decode(&mut &state_update.inputs[2][..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;

        let state_object: PropertyOf<T> = Decode::decode(&mut &state_update.inputs[3][..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;

        Ok(StateUpdate {
            block_number,
            deposit_contract_address: deposit_address,
            range,
            state_object,
        })
    }

    pub fn deserialize_checkpoint(checkpoint: &PropertyOf<T>) -> DispatchResultT<CheckpointOf<T>> {
        let state_update: PropertyOf<T> = Decode::decode(&mut &checkpoint.inputs[0][..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;
        Ok(Checkpoint { state_update })
    }
}
