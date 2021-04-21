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

    type LiquidatorModuleId: Get<ModuleId>;
}

// 3. Storage
decl_storage! {
    trait Store for Module<T: Config> as Vault {
        /// RTokens are minted based on the original asset
        RTokens: map hasher(blake2_128_concat) T::AssetId => T::AssetId;
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
        NotLiquidator,
        // Insufficient balance to deposit
        InsufficientBalance,
        // Zero Amount
        ZeroAmount,
        // Non-registered asset
        NotRegistered,
        // Not enough r tokens to be burnt
        InsufficientSupply,
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

        const LiquidatorModuleId: ModuleId = T::LiquidatorModuleId::get();

        #[weight = 700_000]
        pub fn vault_deposit(origin, asset_id: T::AssetId, amount: T::Balance) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;

            // Clone sender variable to avoid giving ownership when using as a paramter
            let origin_account = sender.clone();
            // Get the balance of the asset that belongs to the sender
            let origin_balance = <Assets::Module<T>>::balance(asset_id, sender);
            
            // Deposit amount cannot be zero
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            // Balance cannot be less than deposit amount
            ensure!(origin_balance >= amount, Error::<T>::InsufficientBalance);

            let mint_amount = Self::calculate_mint_amount(asset_id, amount);
            // Self::deposit(origin, asset_id, amount);

            // Deposit asset into vault
            <Assets::Pallet<T>>::transfer(origin, asset_id, Self::account_id(), amount);

            // Mint rTokens for user
            let mint_token = RTokens::<T>::get(asset_id);
            <Assets::Pallet<T>>::mint(Self::account_id(), mint_token, sender, amount);

            // Emit an event that the deposit went through.
            Self::deposit_event(RawEvent::VaultDeposit(origin_account, amount));
        }

        // @param origin The user calling withdraw
        // @param asset_id The id of the withdraw token
        // @param amount The amount of r token to burn
        #[weight = 700_000]
        pub fn vault_withdraw(origin, asset_id: T::AssetId, amount: T::Balance) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;
            let origin_account = sender.clone();
            let origin_map = (asset_id, sender.clone());

            let rtoken_total_supply: T::Balance = <Assets::Module<T>>::total_supply(RTokens::<T>::get(asset_id));
            ensure!(!rtoken_total_supply.is_zero(), Error::<T>::InsufficientSupply);
            // How much to withdraw based on burn amount
            let withdraw_amount = Self::calculate_withdraw_amount(asset_id, amount, rtoken_total_supply);
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

            // Burn r token
            <Assets::Module<T>>::burn(Self::account_id(), RTokens::<T>::get(asset_id), origin_account, amount);

			<Assets::Module<T>>::transfer(Self::account_id(), asset_id, origin_account, withdraw_amount);

            // Emit an event that the withdraw went through.
            Self::deposit_event(RawEvent::VaultWithdraw(sender.clone(), amount));
        }

        #[weight = 700_000]
        pub fn borrow(origin, asset_id: T::AssetId, amount: T::Balance) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;
            let origin_account = sender.clone();
            let origin_balance = <Assets::Module<T>>::balance(asset_id, Self::account_id());

            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(origin_balance >= amount, Error::<T>::ExceedWithdrawAmount);
            ensure!(origin_account == Self::liquidator_account_id(), Error::<T>::NotLiquidator);
        
            // Transfer asset to liquidator
            // Self::withdraw(sender, asset_id, amount);
			// <Assets::Module<T>>::transfer(Self::account_id(), asset_id, Self::liquidator_account_id(), amount);
        }

        // Register new r token to asset
        #[weight = 700_000]
        pub fn register(origin, asset_id: T::AssetId, r_asset_id: T::AssetId) {
            let sender = ensure_signed(origin)?;

            RTokens::<T>::insert(asset_id, r_asset_id);
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

    pub fn liquidator_account_id() -> T::AccountId {
		T::LiquidatorModuleId::get().into_account()
	}

    fn calculate_mint_amount(asset_id: T::AssetId, amount: T::Balance) -> T::Balance {
        let initial_balance: T::Balance = <Assets::Module<T>>::balance(asset_id, Self::account_id()) - amount;
        let rtoken_total_supply: T::Balance = <Assets::Module<T>>::total_supply(RTokens::<T>::get(asset_id));
        if rtoken_total_supply.is_zero() {
            return amount;
        }
        // mint_amount = amount_deposited * r_pool / pool
        return amount * rtoken_total_supply / initial_balance;
    }

    fn calculate_withdraw_amount(asset_id: T::AssetId, amount: T::Balance, rtoken_total_supply: T::Balance) -> T::Balance {
        let initial_balance: T::Balance = <Assets::Module<T>>::balance(asset_id, Self::account_id());
        // withdraw_amount = amount_to_burn * total / pool
        return amount * initial_balance / rtoken_total_supply;
    }


	fn deposit(origin: T::Origin, asset_id: T::AssetId, amount: T::Balance) {
		// Transfer asset to vault
		// <Assets::Module<T>>::transfer(origin, asset_id, Self::account_id(), amount);
	}

	fn withdraw(sender: T::AccountId, asset_id: T::AssetId, amount: T::Balance) {
		// Transfer asset to vault
		// <Assets::Module<T>>::transfer(Self::account_id(), asset_id, sender, amount);
	}
}