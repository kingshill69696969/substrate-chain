#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::{Get}};
use frame_system::{ensure_signed};
use sp_runtime::{
	traits::{
		Zero, AccountIdConversion
	},
	ModuleId,
};

use pallet_assets as Assets;

pub trait Config: Assets::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    
    type LiquidatorModuleId: Get<ModuleId>;
}

// 3. Storage
decl_storage! {
    trait Store for Module<T: Config> as LiquidatorAdapter {
    }
}

// 4. Events
decl_event! {
    pub enum Event<T> where AssetId = <T as Assets::Config>::AssetId, 
    Balance = <T as Assets::Config>::Balance  {
        /// Event emitted when liquidation occurs
        Liquidated(AssetId, Balance, AssetId, Balance),
    }
}

// 5. Errors
decl_error! {
    pub enum Error for Module<T: Config> {
        /// Non existing asset
        ExistingAsset,
        NotLiquidator,
    }
}

// 6. Callable Functions
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        const LiquidatorModuleId: ModuleId = T::LiquidatorModuleId::get();

        #[weight = 700_000]
        pub fn liquidate(origin, target_user: T::AccountId, pay_asset_id: T::AssetId, get_asset_id: T::AssetId, pay_asset_amount: T::Balance) {
            // TODO: Execute strategy
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::liquidator_account_id(), Error::<T>::NotLiquidator);
            let get_asset_amount: T::Balance = 2;
            Self::deposit_event(RawEvent::Liquidated(pay_asset_id, pay_asset_amount, get_asset_id, get_asset_amount));
        }

        #[weight = 700_000]
        fn asset_price_adapter(origin, asset_id: T::AssetId) -> T::Balance {
            /// TODO: Add offchain price fetch
            2 as u64
        }
    }
}

impl<T: Config> Module<T> {
    pub fn liquidator_account_id() -> T::AccountId {
		T::LiquidatorModuleId::get().into_account()
	}
}