#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String,
};

// -------------------------------------------------------------------
//  Storage Keys
// -------------------------------------------------------------------

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Token balance for a given address
    Balance(Address),
    /// Total supply of tokens
    TotalSupply,
    /// Token name
    Name,
    /// Token symbol
    Symbol,
    /// Admin address (can mint tokens)
    Admin,
}

// -------------------------------------------------------------------
//  Errors
// -------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TokenError {
    /// Insufficient balance for transfer
    InsufficientBalance = 1,
    /// Amount must be greater than zero
    ZeroAmount = 2,
    /// Only admin can mint tokens
    Unauthorized = 3,
    /// Contract has already been initialized
    AlreadyInitialized = 4,
}

// -------------------------------------------------------------------
//  Contract
// -------------------------------------------------------------------

#[contract]
pub struct SimpleToken;

#[contractimpl]
impl SimpleToken {

    // ---------------------------------------------------------------
    //  Init
    // ---------------------------------------------------------------

    /// Initialize the token with name, symbol, and admin address.
    /// The admin is the only address that can mint new tokens.
    pub fn initialize(
        env: Env,
        admin: Address,
        name: String,
        symbol: String,
    ) -> Result<(), TokenError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(TokenError::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::TotalSupply, &0i128);

        env.events()
            .publish((symbol_short!("init"),), (admin.clone(), name.clone(), symbol.clone()));

        Ok(())
    }

    // ---------------------------------------------------------------
    //  Mint
    // ---------------------------------------------------------------

    /// Mint `amount` tokens to `to` address.
    /// Only the admin can call this function.
    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), TokenError> {
        if amount <= 0 {
            return Err(TokenError::ZeroAmount);
        }

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap();

        admin.require_auth();

        let balance = Self::balance(env.clone(), to.clone());
        let new_balance = balance + amount;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &new_balance);

        let total_supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &(total_supply + amount));

        env.events()
            .publish((symbol_short!("mint"),), (&to, amount));

        Ok(())
    }

    // ---------------------------------------------------------------
    //  Transfer
    // ---------------------------------------------------------------

    /// Transfer `amount` tokens from `from` to `to`.
    /// The `from` address must authorize this transaction.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        if amount <= 0 {
            return Err(TokenError::ZeroAmount);
        }

        from.require_auth();

        let from_balance = Self::balance(env.clone(), from.clone());
        if from_balance < amount {
            return Err(TokenError::InsufficientBalance);
        }

        let to_balance = Self::balance(env.clone(), to.clone());

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(from_balance - amount));

        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &(to_balance + amount));

        env.events()
            .publish((symbol_short!("transfer"),), (&from, &to, amount));

        Ok(())
    }

    // ---------------------------------------------------------------
    //  Burn
    // ---------------------------------------------------------------

    /// Burn `amount` tokens from `from` address.
    /// The `from` address must authorize this transaction.
    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        if amount <= 0 {
            return Err(TokenError::ZeroAmount);
        }

        from.require_auth();

        let balance = Self::balance(env.clone(), from.clone());
        if balance < amount {
            return Err(TokenError::InsufficientBalance);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));

        let total_supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &(total_supply - amount));

        env.events()
            .publish((symbol_short!("burn"),), (&from, amount));

        Ok(())
    }

    // ---------------------------------------------------------------
    //  Read-only queries
    // ---------------------------------------------------------------

    /// Get the token balance for an address.
    pub fn balance(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(account))
            .unwrap_or(0)
    }

    /// Get the total supply of tokens.
    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    /// Get the token name.
    pub fn name(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Name)
            .unwrap()
    }

    /// Get the token symbol.
    pub fn symbol(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Symbol)
            .unwrap()
    }

    /// Get the admin address.
    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap()
    }
}

// ===================================================================
//  Tests
// ===================================================================
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup() -> (Env, SimpleTokenClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SimpleToken, ());
        let client = SimpleTokenClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        
        client.initialize(
            &admin,
            &String::from_str(&env, "Test Token"),
            &String::from_str(&env, "TEST"),
        );
        
        (env, client, admin, user)
    }

    #[test]
    fn test_initialize() {
        let (env, client, admin, _user) = setup();
        
        assert_eq!(client.name(), String::from_str(&env, "Test Token"));
        assert_eq!(client.symbol(), String::from_str(&env, "TEST"));
        assert_eq!(client.admin(), admin);
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn test_mint() {
        let (_env, client, _admin, user) = setup();
        
        client.mint(&user, &1000);
        
        assert_eq!(client.balance(&user), 1000);
        assert_eq!(client.total_supply(), 1000);
    }

    #[test]
    fn test_mint_multiple_times() {
        let (_env, client, _admin, user) = setup();
        
        client.mint(&user, &500);
        client.mint(&user, &300);
        
        assert_eq!(client.balance(&user), 800);
        assert_eq!(client.total_supply(), 800);
    }

    #[test]
    fn test_transfer() {
        let (_env, client, _admin, user) = setup();
        let recipient = Address::generate(&_env);
        
        client.mint(&user, &1000);
        client.transfer(&user, &recipient, &400);
        
        assert_eq!(client.balance(&user), 600);
        assert_eq!(client.balance(&recipient), 400);
        assert_eq!(client.total_supply(), 1000);
    }

    #[test]
    fn test_transfer_full_balance() {
        let (_env, client, _admin, user) = setup();
        let recipient = Address::generate(&_env);
        
        client.mint(&user, &1000);
        client.transfer(&user, &recipient, &1000);
        
        assert_eq!(client.balance(&user), 0);
        assert_eq!(client.balance(&recipient), 1000);
    }

    #[test]
    fn test_burn() {
        let (_env, client, _admin, user) = setup();
        
        client.mint(&user, &1000);
        client.burn(&user, &300);
        
        assert_eq!(client.balance(&user), 700);
        assert_eq!(client.total_supply(), 700);
    }

    #[test]
    fn test_burn_full_balance() {
        let (_env, client, _admin, user) = setup();
        
        client.mint(&user, &1000);
        client.burn(&user, &1000);
        
        assert_eq!(client.balance(&user), 0);
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn test_zero_amount_errors() {
        let (_env, client, _admin, user) = setup();
        
        let result = client.try_mint(&user, &0);
        assert!(result.is_err());
        
        let result = client.try_transfer(&user, &user, &0);
        assert!(result.is_err());
    }

    #[test]
    fn test_insufficient_balance_errors() {
        let (_env, client, _admin, user) = setup();
        let recipient = Address::generate(&_env);
        
        client.mint(&user, &100);
        
        let result = client.try_transfer(&user, &recipient, &200);
        assert!(result.is_err());
        
        let result = client.try_burn(&user, &200);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_users() {
        let (_env, client, _admin, user1) = setup();
        let user2 = Address::generate(&_env);
        let user3 = Address::generate(&_env);
        
        client.mint(&user1, &1000);
        client.mint(&user2, &2000);
        
        client.transfer(&user1, &user3, &300);
        client.transfer(&user2, &user3, &500);
        
        assert_eq!(client.balance(&user1), 700);
        assert_eq!(client.balance(&user2), 1500);
        assert_eq!(client.balance(&user3), 800);
        assert_eq!(client.total_supply(), 3000);
    }
}
