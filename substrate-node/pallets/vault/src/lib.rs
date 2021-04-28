#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::{Get, Currency}, Parameter, PalletId};
use frame_system::{ensure_signed};
use codec::{HasCompact};
use sp_runtime::{
	traits::{
		Zero, AccountIdConversion, AtLeast32BitUnsigned, Member, StaticLookup, LookupError
	}, MultiAddress,
};
use frame_support::traits::tokens::fungibles::{Inspect, Mutate, Transfer};

type BalanceOf<T> = <<T as Config>::Currencies as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
type AssetId<T> = <<T as Config>::Currencies as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The vault's module id, used for deriving its sovereign account ID.
    type PalletId: Get<PalletId>;

    type LiquidatorPalletId: Get<PalletId>;

    type Currencies: Inspect<Self::AccountId> + Mutate<Self::AccountId> + Transfer<Self::AccountId>;
}

// 3. Storage
decl_storage! {
    trait Store for Module<T: Config> as Vault {
        /// RTokens are minted based on the original asset
        RTokens: map hasher(blake2_128_concat) AssetId<T> => AssetId<T>;
    }
}

// 4. Events
decl_event! {
    pub enum Event<T> where 
        AccountId = <T as frame_system::Config>::AccountId ,
        Balance = BalanceOf<T>,
    {
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

		const PalletId: PalletId = T::PalletId::get();

        const LiquidatorPalletId: PalletId = T::LiquidatorPalletId::get();

        #[weight = 700_000]
        pub fn vault_deposit(origin, asset_id: AssetId<T>, amount: BalanceOf<T>) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender_origin = origin.clone();
            let sender = ensure_signed(origin)?;

            // Clone sender variable to avoid giving ownership when using as a paramter
            let origin_account = sender.clone();
            // Get the balance of the asset that belongs to the sender
            let origin_balance = T::Currencies::balance(asset_id, &sender);
            
            // Deposit amount cannot be zero
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            // Balance cannot be less than deposit amount
            ensure!(origin_balance >= amount, Error::<T>::InsufficientBalance);

            let mint_amount = Self::calculate_mint_amount(asset_id, amount);
            // Self::deposit(origin, asset_id, amount);

            // Deposit asset into vault
            T::Currencies::transfer(asset_id, &sender, &Self::account_id(), amount, false);

            // Mint rTokens for user
            let mint_token = RTokens::<T>::get(asset_id);
            T::Currencies::mint_into(mint_token, &sender, amount);

            // Emit an event that the deposit went through.
            Self::deposit_event(RawEvent::VaultDeposit(origin_account, amount));
        }

        // @param origin The user calling withdraw
        // @param asset_id The id of the withdraw token
        // @param amount The amount of r token to burn
        #[weight = 700_000]
        pub fn vault_withdraw(origin, asset_id: AssetId<T>, amount: BalanceOf<T>) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;
            let origin_account = sender.clone();
            let origin_map = (asset_id, sender.clone());

            let rtoken_total_supply: BalanceOf<T> = T::Currencies::total_issuance(RTokens::<T>::get(asset_id));
            ensure!(!rtoken_total_supply.is_zero(), Error::<T>::InsufficientSupply);
            // How much to withdraw based on burn amount
            let withdraw_amount = Self::calculate_withdraw_amount(asset_id, amount, rtoken_total_supply);
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

            // Burn r token
            T::Currencies::burn_from(RTokens::<T>::get(asset_id), &sender.clone(), amount);

			T::Currencies::transfer(asset_id, &Self::account_id(), &sender, withdraw_amount, false);

            // Emit an event that the withdraw went through.
            Self::deposit_event(RawEvent::VaultWithdraw(sender.clone(), amount));
        }

        #[weight = 700_000]
        pub fn borrow(origin, asset_id: AssetId<T>, amount: BalanceOf<T>) {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let sender = ensure_signed(origin)?;
            let origin_account = sender.clone();
            let origin_balance = T::Currencies::balance(asset_id, &Self::account_id());

            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
            ensure!(origin_balance >= amount, Error::<T>::ExceedWithdrawAmount);
            ensure!(origin_account == Self::liquidator_account_id(), Error::<T>::NotLiquidator);
        
            // Transfer asset to liquidator
            // Self::withdraw(sender, asset_id, amount);
			// <T::Currencies::Module<T>>::transfer(Self::account_id(), asset_id, Self::liquidator_account_id(), amount);
        }

        // Register new r token to asset
        #[weight = 700_000]
        pub fn register(origin, asset_id: AssetId<T>, r_asset_id: AssetId<T>) {
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
		T::PalletId::get().into_account()
	}

    pub fn liquidator_account_id() -> T::AccountId {
		T::LiquidatorPalletId::get().into_account()
	}

    fn calculate_mint_amount(asset_id: AssetId<T>, amount: BalanceOf<T>) -> BalanceOf<T> {
        let initial_balance: BalanceOf<T> = T::Currencies::balance(asset_id, &Self::account_id()) - amount;
        let rtoken_total_supply: BalanceOf<T> = T::Currencies::total_issuance(RTokens::<T>::get(asset_id));
        if rtoken_total_supply.is_zero() {
            return amount;
        }
        // mint_amount = amount_deposited * r_pool / pool
        return amount * rtoken_total_supply / initial_balance;
    }

    fn calculate_withdraw_amount(asset_id: AssetId<T>, amount: BalanceOf<T>, rtoken_total_supply: BalanceOf<T>) -> BalanceOf<T> {
        let initial_balance: BalanceOf<T> = T::Currencies::balance(asset_id, &Self::account_id());
        // withdraw_amount = amount_to_burn * total / pool
        return amount * initial_balance / rtoken_total_supply;
    }
}
