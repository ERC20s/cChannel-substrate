#![cfg(test)]

use super::*;
use crate::{Module};
use crate::traits::Trait;
use frame_support::{
    impl_outer_event, impl_outer_origin, impl_outer_dispatch,
    parameter_types, weights::Weight
};
use frame_system as system;
use pallet_balances;
use sp_core::{sr25519, Pair, H256};
use sp_runtime::testing::Header;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup, Convert};
use sp_runtime::Perbill;
use pallet_contracts::{
    ContractAddressFor, TrieIdGenerator, 
    TrieId, AccountCounter, CodeHash
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestRuntime;

pub(crate) type Moment = u64;
pub(crate) type AccountId = sr25519::Public;
pub(crate) type Balance = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Signature = sr25519::Signature;


pub mod celer {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        celer<T>,
        pallet_balances<T>,
        system<T>,
        pallet_contracts<T>,
    }
}

impl_outer_dispatch! {
    pub enum Call for TestRuntime where origin: Origin {
        frame_system::System,
        celer_pay::CelerPayModule,
        mock_boolean_condition::MockBooleanCondition,
    }
}

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const ExistentialDeposit: u64 = 1; // should be greater than zero
}

impl frame_system::Trait for TestRuntime {
    /// The basic call filter to use in dispatchable.
	type BaseCallFilter = ();
    /// The identifier used to distinguish between accounts.
	type AccountId = sr25519::Public;
    /// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
    /// The index type for storing how many extrinsics an account has signed.
	type Index = u64;
    /// The index type for blocks.
	type BlockNumber = u64;
    /// The type for hashing blocks and tries.
	type Hash = H256;
    /// The hashing algorithm used.
	type Hashing = BlakeTwo256;
    /// The header type.
	type Header = Header;
    /// The ubiquitous event type.
	type Event = TestEvent;
    /// The ubiquitous origin type.
	type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
    /// The weight of database operations that the runtime can invoke.
	type DbWeight = ();
    /// The weight of the overhead invoked on the block import process, independent of the
	/// extrinsics included in that block.
	type BlockExecutionWeight = ();
    /// The base weight of any extrinsic processed by the runtime, independent of the
	/// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
	type ExtrinsicBaseWeight = ();
    /// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
	/// idependent of the logic of that extrinsics. (Roughly max block weight - average on
	/// initialize cost).
	type MaximumExtrinsicWeight = MaximumBlockWeight;
    /// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Version of the runtime.
	type Version = ();
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = ();
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<u64>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_balances::Trait for TestRuntime {
    type MaxLocks = ();
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<TestRuntime>;
    type WeightInfo = ();
}

impl pallet_timestamp::Trait for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
	pub const SignedClaimHandicap: u64 = 2;
	pub const TombstoneDeposit: u64 = 16;
	pub const StorageSizeOffset: u32 = 8;
	pub const RentByteFee: u64 = 4;
	pub const RentDepositOffset: u64 = 10_000;
	pub const SurchargeReward: u64 = 150;
	pub const MaxDepth: u32 = 100;
	pub const MaxValueSize: u32 = 16_384;
}

parameter_types! {
	pub const TransactionByteFee: u64 = 0;
}

impl Convert<Weight, BalanceOf<Self>> for TestRuntime {
	fn convert(w: Weight) -> BalanceOf<Self> {
		w
	}
}

impl pallet_contracts::Trait for TestRuntime {
	type Time = Timestamp;
	type Randomness = Randomness;
	type Currency = Balances;
	type DetermineContractAddress =  DummyContractAddressFor;
	type Event = TestEvent;
	type TrieIdGenerator = DummyTrieIdGenerator;
	type RentPayment = ();
	type SignedClaimHandicap = SignedClaimHandicap;
	type TombstoneDeposit = TombstoneDeposit;
	type StorageSizeOffset = StorageSizeOffset;
	type RentByteFee = RentByteFee;
	type RentDepositOffset = RentDepositOffset;
	type SurchargeReward = SurchargeReward;
	type MaxDepth = MaxDepth;
	type MaxValueSize = MaxValueSize;
	type WeightPrice = Self;
}

impl mock_boolean_condition::Trait for TestRuntime {}

impl mock_numeric_condition::Trait for TestRuntime {}

impl Trait for TestRuntime {
    type Currency = pallet_balances::Module<Self>;
    type Event = TestEvent;
    type Public = sr25519::Public;
    type Signature = sr25519::Signature;
    type Call = Call;
}

pub type CelerPayModule = Module<TestRuntime>;
pub type System = frame_system::Module<TestRuntime>;
pub type Timestamp = pallet_timestamp::Module<TestRuntime>;
type MockBooleanCondition = mock_boolean_condition::Module<TestRuntime>;
type Balances = pallet_balances::Module<TestRuntime>;
type Randomness = pallet_randomness_collective_flip::Module<TestRuntime>;

pub struct DummyContractAddressFor;
impl ContractAddressFor<H256, sr25519::Public> for DummyContractAddressFor {
	fn contract_address_for(_code_hash: &H256, _data: &[u8], origin: &sr25519::Public) -> sr25519::Public {
		*origin
	}
}

pub struct DummyTrieIdGenerator;
impl TrieIdGenerator<sr25519::Public> for DummyTrieIdGenerator {
	fn trie_id(account_id: &sr25519::Public) -> TrieId {
		let new_seed = AccountCounter::mutate(|v| {
			*v = v.wrapping_add(1);
			*v
		});

		let mut res = vec![];
		res.extend_from_slice(&new_seed.to_le_bytes());
		//res.extend_from_slice(&account_id.to_le_bytes());
		res
	}
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let alice: sr25519::Public = account_key("Alice");
        let bob: sr25519::Public = account_key("Bob");
        let risa: sr25519::Public = account_key("Risa");
        let carl: sr25519::Public = account_key("Carl");

        let mut t = system::GenesisConfig::default()
            .build_storage::<TestRuntime>().unwrap();
        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: vec![(alice, 1000), (bob, 1000), (risa, 1000), (carl, 100000)],
        }.assimilate_storage(&mut t).unwrap();
        sp_io::TestExternalities::new(t)
    }
}

pub(crate) fn account_pair(s: &str) -> sr25519::Pair {
    sr25519::Pair::from_string(&format!("//{}", s), None).expect("static values are valid: qed")
}

pub(crate) fn account_key(s: &str) -> sr25519::Public {
    sr25519::Pair::from_string(&format!("//{}", s), None)
        .expect("static values are valid; qed")
        .public()
}
