use frame_support::{ dispatch::{DispatchError, DispatchResult}};
pub trait Assets<T: {Origin, AssetId, Source, Balance}> {
    fn transfer(
        origin: T::Origin,
        id: T::AssetId,
        target: T::Source,
        amount: T::Balance,
    ) -> DispatchResult;

    fn balance(
        id: T::AssetId,
        who: T::AccountId,
    ) -> T::Balance;

    fn total_supply(
        id: T::AssetId,
    ) -> T::Balance;

    fn mint(
        origin: T::Origin,
        id: T::AssetId,
        beneficiary: T::Source,
        amount: T::Balance,
    ) -> DispatchResult;

    fn burn(
        origin: T::Origin,
        id: T::AssetId,
        who: T::Source,
        amount: T::Balance,
    ) -> DispatchResult;
}