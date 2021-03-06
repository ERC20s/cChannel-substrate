use super::{BalanceOf, Error, Module as CelerPayModule, RawEvent};
use crate::traits::Trait;
use crate::pay_registry::PayRegistry;
use crate::numeric_condition_caller::NumericConditionCaller;
use codec::{Decode, Encode};
use frame_support::{ensure};
use frame_system::{self as system};
use pallet_timestamp;
use sp_runtime::traits::{CheckedAdd, Hash, Zero, Dispatchable};
use sp_runtime::{ModuleId, RuntimeDebug, DispatchError};
use sp_std::{vec::Vec, boxed::Box};

pub const PAY_RESOLVER_ID: ModuleId = ModuleId(*b"Resolver");

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub enum ConditionType {
    HashLock,
    BooleanRuntimeModule,
    NumericRuntimeModule, 
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct Condition<Hash, Call> {
    pub condition_type: ConditionType,
    pub hash_lock: Option<Hash>,
    pub call_is_finalized: Option<Box<Call>>, // overarching call is_finalized of boolean runtime module
    pub call_get_outcome: Option<Box<Call>>, // overarching call get_outcome of boolean runtime module
    pub numeric_app_num: Option<u32>, // number of registered numeric app 
    pub numeric_session_id: Option<Hash>, // session id of numeric condition
    pub args_query_finalzation: Option<Vec<u8>>, // the encoded query finalization of numeric runtime module
    pub args_query_outcome: Option<Vec<u8>>, // the encoded query outcome of numeric runtime module
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub enum TransferFunctionType {
    BooleanAnd,
    BooleanOr,
    BooleanCircut,
    NumericAdd,
    NumericMax,
    NumericMin,
}

// Currently native token is only supoorted.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub enum TokenType {
    Invalid,
    Celer, // native token. If Kusama network,change from Celer to Ksm.
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct TokenInfo {
    pub token_type: TokenType,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AccountAmtPair<AccountId, Balance> {
    pub account: Option<AccountId>,
    pub amt: Balance,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct TokenTransfer<AccountId, Balance> {
    pub token: TokenInfo,
    pub receiver: AccountAmtPair<AccountId, Balance>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct TransferFunction<AccountId, Balance> {
    pub logic_type: TransferFunctionType,
    pub max_transfer: TokenTransfer<AccountId, Balance>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct ConditionalPay<Moment, BlockNumber, AccountId, Hash, Call, Balance> {
    pub pay_timestamp: Moment,
    pub src: AccountId,
    pub dest: AccountId,
    pub conditions: Vec<Condition<Hash, Call>>,
    pub transfer_func: TransferFunction<AccountId, Balance>,
    pub resolve_deadline: BlockNumber,
    pub resolve_timeout: BlockNumber,
}

pub type ConditionalPayOf<T> = ConditionalPay<
    <T as pallet_timestamp::Trait>::Moment,
    <T as system::Trait>::BlockNumber,
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as Trait>::Call,
    BalanceOf<T>,
>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct ResolvePaymentConditionsRequest<Moment, BlockNumber, AccountId, Hash, Call, Balance> {
    pub cond_pay: ConditionalPay<Moment, BlockNumber, AccountId, Hash, Call, Balance>,
    pub hash_preimages: Vec<Hash>,
}

pub type ResolvePaymentConditionsRequestOf<T> = ResolvePaymentConditionsRequest<
    <T as pallet_timestamp::Trait>::Moment,
    <T as system::Trait>::BlockNumber,
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as Trait>::Call,
    BalanceOf<T>,
>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct CondPayResult<Moment, BlockNumber, AccountId, Hash, Call, Balance> {
    pub cond_pay: ConditionalPay<Moment, BlockNumber, AccountId, Hash, Call, Balance>,
    pub amount: Balance,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct VouchedCondPayResult<Moment, BlockNumber, AccountId, Hash, Call, Balance, Signature> {
    pub cond_pay_result: CondPayResult<Moment, BlockNumber, AccountId, Hash, Call, Balance>,
    pub sig_of_src: Signature,
    pub sig_of_dest: Signature,
}

pub type VouchedCondPayResultOf<T> = VouchedCondPayResult<
    <T as pallet_timestamp::Trait>::Moment,
    <T as system::Trait>::BlockNumber,
    <T as system::Trait>::AccountId,
    <T as system::Trait>::Hash,
    <T as Trait>::Call,
    BalanceOf<T>,
    <T as Trait>::Signature,
>;

pub struct PayResolver<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> PayResolver<T> {
    // Resolve a payment by onchain getting its condition outcomes
    pub fn resolve_payment_by_conditions(
        resolve_pay_request: ResolvePaymentConditionsRequestOf<T>,
    ) -> Result<(T::Hash, BalanceOf<T>, T::BlockNumber), DispatchError> {
        let pay = resolve_pay_request.cond_pay;

        let mut amount: BalanceOf<T> = Zero::zero();
        let func_type = pay.transfer_func.logic_type.clone();
        if func_type == TransferFunctionType::BooleanAnd {
            amount = calculate_boolean_and_payment::<T>(
                pay.clone(),
                resolve_pay_request.hash_preimages,
            )?;
        } else if func_type == TransferFunctionType::BooleanOr {
            amount = calculate_boolean_or_payment::<T>(
                pay.clone(), 
                resolve_pay_request.hash_preimages
            )?;
        } else if is_numeric_logic::<T>(func_type.clone()) {
            amount = calculate_numeric_logic_payment::<T>(
                pay.clone(),
                resolve_pay_request.hash_preimages,
                func_type.clone(),
            )?;
        } else {
            Err(Error::<T>::Error)?
        }

        let encoded = encode_conditional_pay::<T>(pay.clone());
        let pay_hash = T::Hashing::hash(&encoded);
        return resolve_payment::<T>(pay, pay_hash, amount);
    }

    // Resolve a payment by submitting an offchain vouched result
    pub fn resolve_payment_vouched_result(
        vouched_pay_result: VouchedCondPayResultOf<T>,
    ) -> Result<(T::Hash, BalanceOf<T>, T::BlockNumber), DispatchError> {
        let pay_result = vouched_pay_result.cond_pay_result;
        let pay = pay_result.cond_pay;

        ensure!(
            pay_result.amount <= pay.transfer_func.max_transfer.receiver.amt,
            "Exceed max transfer amount"
        );

        // Check signatures
        let encoded = encode_conditional_pay::<T>(pay.clone());
        CelerPayModule::<T>::check_single_signature(
            vouched_pay_result.sig_of_src,
            &encoded,
            pay.src.clone(),
        )?;
        CelerPayModule::<T>::check_single_signature(
            vouched_pay_result.sig_of_dest,
            &encoded,
            pay.dest.clone(),
        )?;

        let pay_hash = T::Hashing::hash(&encoded);
        return resolve_payment::<T>(pay, pay_hash, pay_result.amount);
    }
}

fn resolve_payment<T: Trait>(
    pay: ConditionalPayOf<T>,
    pay_hash: T::Hash,
    amount: BalanceOf<T>,
) -> Result<(T::Hash, BalanceOf<T>, T::BlockNumber), DispatchError> {
    let block_number = frame_system::Module::<T>::block_number();
    ensure!(
        block_number <= pay.resolve_deadline,
        "Passed pay resolve deadline in cond_pay msg"
    );

    let pay_id = calculate_pay_id::<T>(pay_hash);
    let pay_info: (BalanceOf<T>, T::BlockNumber) = PayRegistry::<T>::get_pay_info(pay_id)?;
    let current_amt = pay_info.0;
    let current_deadline = pay_info.1;

    // Should never resolve a pay before or not rearching on-chain resolve deadline.
    ensure!(
        current_deadline.is_zero() || block_number <= current_deadline,
        "Passed onchain resolve pay deadline"
    );

    if current_deadline > Zero::zero() {
        // current_deadline > 0 implies that this pay ha been updated
        // payment amount must be monotone increasing
        ensure!(amount > current_amt, "New amount is not larger");

        if amount == pay.transfer_func.max_transfer.receiver.amt {
            // set resolve deadline = current block number if amount = max
            PayRegistry::<T>::set_pay_info(pay_hash, amount, block_number)?;
            
            // Emit ResolvePayment event
            CelerPayModule::<T>::deposit_event(RawEvent::ResolvePayment(
                pay_id,
                amount,
                block_number
            ));
            return Ok((pay_id, amount, block_number));
        } else {
            // should not update the onchain resolve deadline if not max amount
            PayRegistry::<T>::set_pay_amount(pay_hash, amount)?;
            
            // Emit ResolvePayment event
            CelerPayModule::<T>::deposit_event(RawEvent::ResolvePayment(
                pay_id,
                amount,
                current_deadline
            ));
            return Ok((pay_id, amount, current_deadline));
        }
    } else {
        let new_deadline: T::BlockNumber;
        if amount == pay.transfer_func.max_transfer.receiver.amt {
            new_deadline = block_number.clone();
        } else {
            let timeout = block_number.checked_add(&pay.resolve_timeout).ok_or(Error::<T>::OverFlow)?;
            if timeout < pay.resolve_deadline {
                new_deadline = block_number.checked_add(&pay.resolve_timeout).ok_or(Error::<T>::OverFlow)?;
            } else {
                new_deadline = pay.resolve_deadline;
            }
            // 0 is reserved for unresolved status of a payment
            ensure!(new_deadline > Zero::zero(), "New resolve deadline is 0");
        }

        PayRegistry::<T>::set_pay_info(pay_hash, amount, new_deadline)?;
        
        // Emit ResolvePayment event
        CelerPayModule::<T>::deposit_event(RawEvent::ResolvePayment(
            pay_id,
            amount,
            new_deadline
        ));
        return Ok((pay_id, amount, new_deadline));
    }
}

// Calculate the result amount of BooleanAnd payment
fn calculate_boolean_and_payment<T: Trait>(
    pay: ConditionalPayOf<T>,
    preimages: Vec<T::Hash>,
) -> Result<BalanceOf<T>, DispatchError> {
    let mut j: usize = 0;
    let mut has_false_contract_cond: bool = false;
    for i in 0..pay.conditions.len() {
        let cond = pay.conditions[i].clone();
        if cond.condition_type == ConditionType::HashLock {
            let hash_lock = match cond.hash_lock {
                Some(lock) => lock,
                None => Err(Error::<T>::HashLockNotExist)?,
            };
            ensure!(preimages[j] == hash_lock, "Wrong preimage");
            j = j + 1;
        } else if cond.condition_type == ConditionType::BooleanRuntimeModule {
            let pay_resolver_account = CelerPayModule::<T>::get_pay_resolver_id();
            
            // call is_finalized of boolean condition
            let call_is_finalized = match cond.call_is_finalized {
                Some(call) => call,
                None => Err(Error::<T>::CallIsFinalizedNotExist)?,
            };
            let is_finalized = call_is_finalized.dispatch(frame_system::RawOrigin::Signed(pay_resolver_account.clone()).into());
            ensure!(
                is_finalized.is_ok(),
                "Condition is not finalized"
            );

            // call get_outcome of boolean condition
            let call_get_outcome = match cond.call_get_outcome {
                Some(call) => call,
                None => Err(Error::<T>::CallGetOutcomeNotExist)?,
            };
            let outcome = call_get_outcome.dispatch(frame_system::RawOrigin::Signed(pay_resolver_account).into());
            if (!outcome.is_ok()) && (outcome.unwrap_err().error == DispatchError::Other("FalseOutcome")) {
                has_false_contract_cond = true;
            }
        } else {
            Err(Error::<T>::Error)?
        }
    }

    if has_false_contract_cond == true {
        return Ok(Zero::zero());
    } else {
        return Ok(pay.transfer_func.max_transfer.receiver.amt);
    }
}

// Calculate the result amount of BooleanOr payment
fn calculate_boolean_or_payment<T: Trait>(
    pay: ConditionalPayOf<T>,
    preimages: Vec<T::Hash>,
) -> Result<BalanceOf<T>, DispatchError> {
    let mut j: usize = 0;
    // Whether there are any contract based conditions, i.e. DEPLOYED_CONTRACT or VIRTUAL_CONTRACT
    let mut has_contract_cond = false;
    let mut has_true_contract_cond = false;
    for i in 0..pay.conditions.len() {
        let cond = pay.conditions[i].clone();
        if cond.condition_type == ConditionType::HashLock {
            let hash_lock = match cond.hash_lock {
                Some(lock) => lock,
                None => Err(Error::<T>::HashLockNotExist)?,
            };
            ensure!(preimages[j] == hash_lock, "Wrong preimage");
            j += 1;
        } else if cond.condition_type == ConditionType::BooleanRuntimeModule {
            let pay_resolver_account = CelerPayModule::<T>::get_pay_resolver_id();
            
            // call is_finalized of boolean_condition
            let call_is_finalized = match cond.call_is_finalized {
                Some(call) => call,
                None => Err(Error::<T>::CallIsFinalizedNotExist)?,
            };
            let is_finalized = call_is_finalized.dispatch(frame_system::RawOrigin::Signed(pay_resolver_account.clone()).into());
            ensure!(
                is_finalized.is_ok(),
                "Condition is not finalized"
            );
            has_contract_cond = true;

            // call get_outcome of boolean_condition
            let call_get_outcome = match cond.call_get_outcome {
                Some(call) => call,
                None => Err(Error::<T>::CallGetOutcomeNotExist)?,
            };
            let outcome = call_get_outcome.dispatch(frame_system::RawOrigin::Signed(pay_resolver_account).into());
            if outcome.is_ok() {
                has_true_contract_cond = true;
            }
        } else {
            Err(Error::<T>::Error)?
        }
    }

    if has_contract_cond == false || has_true_contract_cond == true {
        return Ok(pay.transfer_func.max_transfer.receiver.amt);
    } else {
        return Ok(Zero::zero());
    }
}

// Calculate the result amount of numeric logic payment, including NUMERIC_ADD, NUMERIC_MAX and NUMERIC_MIN
fn calculate_numeric_logic_payment<T: Trait>(
    pay: ConditionalPayOf<T>,
    preimages: Vec<T::Hash>,
    func_type: TransferFunctionType,
) -> Result<BalanceOf<T>, DispatchError> {
    let mut amount: BalanceOf<T> = <BalanceOf<T>>::zero();
    let mut j: usize = 0;
    let mut has_contract_cond: bool = false;
    for i in 0..pay.conditions.len() {
        let cond = pay.conditions[i].clone();
        if cond.condition_type == ConditionType::HashLock {
            let hash_lock = match cond.hash_lock {
                Some(lock) => lock,
                None => Err(Error::<T>::HashLockNotExist)?,
            };
            ensure!(preimages[j] == hash_lock, "Wrong preimage");
            j = j + 1;
        } else if cond.condition_type == ConditionType::NumericRuntimeModule {
            // the number of registered numeric app
            let numeric_app_number = match cond.numeric_app_num {
                Some(app_num) => app_num,
                None => Err(Error::<T>::NumericAppNotExist)?,
            };
            // session id of numeric condition
            let session_id = match cond.numeric_session_id {
                Some(id) => id,
                None => Err(Error::<T>::NumericSessionIdNotExist)?,
            };

            let is_finalized: bool = NumericConditionCaller::<T>::call_is_finalized(
                numeric_app_number, 
                &session_id, 
                cond.args_query_finalzation
            )?;
            ensure!(is_finalized == true, "Condition is not finalized");

            let outcome: BalanceOf<T> = NumericConditionCaller::<T>::call_get_outcome(
                numeric_app_number, 
                &session_id, 
                cond.args_query_outcome
            )?;
            if func_type == TransferFunctionType::NumericAdd {
                amount = amount + outcome;
            } else if func_type == TransferFunctionType::NumericMax {
                if outcome > amount {
                    amount = outcome;
                }
            } else if func_type == TransferFunctionType::NumericMin {
                if has_contract_cond == true {
                    if outcome < amount {
                        amount = outcome;
                    }
                } else {
                    amount = outcome;
                }
            } else {
                Err(Error::<T>::Error)?
            }

            has_contract_cond = true;
        } else {
            Err(Error::<T>::Error)?
        }
    }

    if has_contract_cond == true {
        ensure!(
            amount <= pay.transfer_func.max_transfer.receiver.amt,
            "Exceed max transfer amount"
        );
        return Ok(amount);
    } else {
        return Ok(pay.transfer_func.max_transfer.receiver.amt);
    }
}

fn is_numeric_logic<T: Trait>(func_type: TransferFunctionType) -> bool {
    return func_type == TransferFunctionType::NumericAdd
        || func_type == TransferFunctionType::NumericMax
        || func_type == TransferFunctionType::NumericMin;
}

// Calculate pay id
pub fn calculate_pay_id<T: Trait>(pay_hash: T::Hash) -> T::Hash {
    let pay_resolver_account = CelerPayModule::<T>::get_pay_resolver_id();
    let mut encoded = pay_hash.encode();
    encoded.extend(pay_resolver_account.encode());
    let pay_id = T::Hashing::hash(&encoded);
    return pay_id;
}

pub fn encode_conditional_pay<T: Trait>(pay: ConditionalPayOf<T>) -> Vec<u8> {
    let mut encoded = pay.pay_timestamp.encode();
    encoded.extend(pay.src.encode());
    encoded.extend(pay.dest.encode());
    pay.conditions.into_iter().for_each(|condition| {
        encoded.extend(condition.condition_type.encode());
        encoded.extend(condition.hash_lock.encode());
        encoded.extend(condition.call_is_finalized.encode());
        encoded.extend(condition.call_get_outcome.encode());
        encoded.extend(condition.numeric_app_num.encode());
        encoded.extend(condition.numeric_session_id.encode());
        encoded.extend(condition.args_query_finalzation.encode());
        encoded.extend(condition.args_query_outcome.encode());
    });
    encoded.extend(pay.transfer_func.logic_type.encode());
    encoded.extend(pay.transfer_func.max_transfer.token.token_type.encode());
    encoded.extend(pay.transfer_func.max_transfer.receiver.account.encode());
    encoded.extend(pay.transfer_func.max_transfer.receiver.amt.encode());
    encoded.extend(pay.resolve_deadline.encode());
    encoded.extend(pay.resolve_timeout.encode());

    return encoded;
}

