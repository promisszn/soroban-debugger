#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env,
};

// -------------------------------------------------------------------
//  Storage Keys
// -------------------------------------------------------------------

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    /// Depositor address
    Depositor,
    /// Beneficiary address
    Beneficiary,
    /// Escrowed amount
    Amount,
    /// Unlock timestamp
    UnlockTime,
    /// Escrow status
    Status,
}

// -------------------------------------------------------------------
//  Escrow Status
// -------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum EscrowStatus {
    /// Funds deposited, waiting for unlock time
    Pending = 0,
    /// Funds released to beneficiary
    Released = 1,
    /// Funds refunded to depositor
    Refunded = 2,
}

// -------------------------------------------------------------------
//  Errors
// -------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    /// Escrow has already been initialized
    AlreadyInitialized = 1,
    /// Escrow has not been initialized
    NotInitialized = 2,
    /// Only depositor can refund
    Unauthorized = 3,
    /// Cannot release before unlock time
    TooEarly = 4,
    /// Escrow already finalized (released or refunded)
    AlreadyFinalized = 5,
    /// Amount must be greater than zero
    ZeroAmount = 6,
}

// -------------------------------------------------------------------
//  Contract
// -------------------------------------------------------------------

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Deposit funds into escrow with a time lock.
    ///
    /// # Arguments
    /// * `depositor` - Address depositing the funds
    /// * `beneficiary` - Address that will receive funds after unlock time
    /// * `amount` - Amount to escrow
    /// * `unlock_time` - Timestamp when funds can be released
    pub fn deposit(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        amount: i128,
        unlock_time: u64,
    ) -> Result<(), EscrowError> {
        if env.storage().instance().has(&DataKey::Status) {
            return Err(EscrowError::AlreadyInitialized);
        }

        if amount <= 0 {
            return Err(EscrowError::ZeroAmount);
        }

        depositor.require_auth();

        env.storage().instance().set(&DataKey::Depositor, &depositor);
        env.storage().instance().set(&DataKey::Beneficiary, &beneficiary);
        env.storage().instance().set(&DataKey::Amount, &amount);
        env.storage().instance().set(&DataKey::UnlockTime, &unlock_time);
        env.storage().instance().set(&DataKey::Status, &EscrowStatus::Pending);

        env.events().publish(
            (symbol_short!("deposit"),),
            (depositor, beneficiary, amount, unlock_time),
        );

        Ok(())
    }

    /// Release funds to the beneficiary after the unlock time has passed.
    pub fn release(env: Env) -> Result<(), EscrowError> {
        let status: EscrowStatus = env
            .storage()
            .instance()
            .get(&DataKey::Status)
            .ok_or(EscrowError::NotInitialized)?;

        if status != EscrowStatus::Pending {
            return Err(EscrowError::AlreadyFinalized);
        }

        let unlock_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::UnlockTime)
            .unwrap();

        let current_time = env.ledger().timestamp();
        if current_time < unlock_time {
            return Err(EscrowError::TooEarly);
        }

        let beneficiary: Address = env
            .storage()
            .instance()
            .get(&DataKey::Beneficiary)
            .unwrap();

        beneficiary.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::Status, &EscrowStatus::Released);

        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();

        env.events().publish(
            (symbol_short!("release"),),
            (beneficiary, amount),
        );

        Ok(())
    }

    /// Refund the depositor before unlock time.
    pub fn refund(env: Env) -> Result<(), EscrowError> {
        let status: EscrowStatus = env
            .storage()
            .instance()
            .get(&DataKey::Status)
            .ok_or(EscrowError::NotInitialized)?;

        if status != EscrowStatus::Pending {
            return Err(EscrowError::AlreadyFinalized);
        }

        let depositor: Address = env
            .storage()
            .instance()
            .get(&DataKey::Depositor)
            .unwrap();

        depositor.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::Status, &EscrowStatus::Refunded);

        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();

        env.events().publish(
            (symbol_short!("refund"),),
            (depositor, amount),
        );

        Ok(())
    }

    /// Get the current status of the escrow.
    /// Returns:
    /// - depositor address
    /// - beneficiary address
    /// - amount
    /// - unlock_time
    /// - status (Pending, Released, or Refunded)
    pub fn get_status(env: Env) -> Result<(Address, Address, i128, u64, EscrowStatus), EscrowError> {
        let status: EscrowStatus = env
            .storage()
            .instance()
            .get(&DataKey::Status)
            .ok_or(EscrowError::NotInitialized)?;

        let depositor: Address = env.storage().instance().get(&DataKey::Depositor).unwrap();
        let beneficiary: Address = env.storage().instance().get(&DataKey::Beneficiary).unwrap();
        let amount: i128 = env.storage().instance().get(&DataKey::Amount).unwrap();
        let unlock_time: u64 = env.storage().instance().get(&DataKey::UnlockTime).unwrap();

        Ok((depositor, beneficiary, amount, unlock_time, status))
    }
}

// ===================================================================
//  Tests
// ===================================================================
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_deposit() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        client.deposit(&depositor, &beneficiary, &1000, &100);

        let (dep, ben, amt, time, status) = client.get_status();
        assert_eq!(dep, depositor);
        assert_eq!(ben, beneficiary);
        assert_eq!(amt, 1000);
        assert_eq!(time, 100);
        assert_eq!(status, EscrowStatus::Pending);
    }

    #[test]
    fn test_release_after_unlock_time() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        client.deposit(&depositor, &beneficiary, &1000, &100);

        // Set ledger timestamp to after unlock time
        env.ledger().with_mut(|li| li.timestamp = 150);

        client.release();

        let (_dep, _ben, _amt, _time, status) = client.get_status();
        assert_eq!(status, EscrowStatus::Released);
    }

    #[test]
    fn test_release_before_unlock_time_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        client.deposit(&depositor, &beneficiary, &1000, &100);

        // Set ledger timestamp to before unlock time
        env.ledger().with_mut(|li| li.timestamp = 50);

        let result = client.try_release();
        assert!(result.is_err());
    }

    #[test]
    fn test_refund() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        client.deposit(&depositor, &beneficiary, &1000, &100);
        client.refund();

        let (_dep, _ben, _amt, _time, status) = client.get_status();
        assert_eq!(status, EscrowStatus::Refunded);
    }

    #[test]
    fn test_cannot_deposit_twice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        client.deposit(&depositor, &beneficiary, &1000, &100);

        let result = client.try_deposit(&depositor, &beneficiary, &500, &200);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_amount_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(&env, &contract_id);

        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        let result = client.try_deposit(&depositor, &beneficiary, &0, &100);
        assert!(result.is_err());
    }
}
