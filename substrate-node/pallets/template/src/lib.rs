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

    /// The vault's module id, used for deriving its sovereign account ID.
    type ModuleId: Get<ModuleId>;
}

// 3. Storage
decl_storage! {
    trait Store for Module<T: Config> as Vault {
        /// The storage item for our deposits.
        /// It maps a user to their balance.
        VaultBalances: map hasher(blake2_128_concat) (T::AssetId, T::AccountId) => T::Balance;
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
        /// Withdrawing more than previously deposited
        ExceedWithdrawAmount,
        /// Borrow called from a non-whitelisted account
        NotWhitelist,
        // Insufficient balance to deposit
        InsufficientBalance,
        // Zero Amount
        ZeroAmount,
    }
}

// 6. Callable Functions
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

		const ModuleId: ModuleId = T::ModuleId::get();

        #[weight = 700_000]
        pub fn vault_deposit(origin, asset_id: T::AssetId, amount: T::Balance) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;

            let origin_balance = <Assets::Module<T>>::balance(asset_id, sender);

            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(origin_balance >= amount, Error::<T>::InsufficientBalance);

            Self::deposit(origin, asset_id, amount);

            VaultBalances::<T>::mutate((asset_id, sender), |balance| *balance += amount);

            // Emit an event that the deposit went through.
            Self::deposit_event(RawEvent::VaultDeposit(sender, amount));
        }

        #[weight = 700_000]
        fn vault_withdraw(origin, asset_id: T::AssetId, amount: T::Balance) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;

            let origin_balance = VaultBalances::<T>::get(&(asset_id, sender));

            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(origin_balance >= amount, Error::<T>::ExceedWithdrawAmount);

            // Transfer asset to vault
			Self::deposit(sender, asset_id, amount);
			VaultBalances::<T>::insert((asset_id, sender), origin_balance - amount);

            // Emit an event that the withdraw went through.
            Self::deposit_event(RawEvent::VaultWithdraw(sender, amount));
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
		T::ModuleId::get().into_account()
	}

	fn deposit(origin: T::Origin, asset_id: T::AssetId, amount: T::Balance) {
		// Transfer asset to vault
		<Assets::Module<T>>::transfer(origin, asset_id, Self::account_id(), amount);
	}

	fn withdraw(sender: T::AccountId, asset_id: T::AssetId, amount: T::Balance) {
		// Transfer asset to vault
		<Assets::Module<T>>::transfer(Self::account_id(), asset_id, sender, amount);
	}
}