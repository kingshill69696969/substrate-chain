#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{AccountIdConversion, Zero},
    ModuleId,
};

use pallet_assets as Assets;
use pallet_vault as Vault;
use pallet_liquidator_adapter as LiquidatorAdapter;
pub trait Config: Assets::Config + Vault::Config + LiquidatorAdapter::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The liquidator's module id, used for deriving its sovereign account ID.
    type LiquidatorModuleId: Get<ModuleId>;
}

// 3. Storage
decl_storage! {
    trait Store for Module<T: Config> as Liquidator {
        pub Finders: Vec<T::AccountId>; 
    }
}

// 4. Events
decl_event! {
    pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId, Balance = <T as Assets::Config>::Balance {
        /// Event emitted when a Backer deposits
        VaultDeposit(AccountId, Balance),
        /// Event emitted when a Backer withdraws
        VaultWithdraw(AccountId, Balance),
    }
}

// 5. Errors
decl_error! {
    pub enum Error for Module<T: Config> {
        /// Borrows more than liquidation returns
        BorrowExceedsLiquidation,
    }
}

// 6. Callable Functions
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        const ModuleId: ModuleId = T::LiquidatorModuleId::get();

        #[weight = 700_000]
        pub fn liquidate(origin, target_user: T::AccountId, pay_asset_id: T::AssetId, get_asset_id: T::AssetId, pay_asset_amount: T::Balance, max_liquidatable: T::Balance, borrow_amount: T::Balance) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;
            let origin_account = sender.clone();

            let pay_asset_price = <LiquidatorAdapter::Module<T>>::asset_price_adapter(pay_asset_id);
            ensure!(pay_asset_price * pay_asset_amount > borrow_amount, Error::<T>::BorrowExceedsLiquidation);
            <Vault::Module<T>>::borrow(frame_system::RawOrigin::Signed(Self::account_id()).into(), pay_asset_id, borrow_amount);
            <LiquidatorAdapter::Module<T>>::liquidate(frame_system::RawOrigin::Signed(Self::account_id()).into(), target_user, pay_asset_id, get_asset_id, pay_asset_amount);
            // Emit an event that the deposit went through.
            Self::deposit_event(RawEvent::VaultDeposit(sender, amount));
        }
    }
}

impl<T: Config> Module<T> {
    // Add public immutables and private mutables.

    /// The account ID of the vault.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        T::LiquidatorModuleId::get().into_account()
    }

    pub fn is_finder(finder: &T::AccountId) -> bool {
        Finders::<T>::get().binary_search(&finder).is_ok()
    }
}
