#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use core::marker::PhantomData;

use codec::{Decode, Encode};

use pallet_balances::NegativeImbalance;
use pallet_commitments::CanCommit;
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};

use frame_support::pallet_prelude::{DispatchError, DispatchResult, Get};
use frame_system::{EnsureNever, EnsureRoot, RawOrigin};

use pallet_registry::CanRegisterIdentity;
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
    crypto::{ByteArray, KeyTypeId},
    OpaqueMetadata, H160, H256, U256,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        AccountIdLookup, BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable,
        IdentifyAccount, NumberFor, One, PostDispatchInfoOf, UniqueSaturatedInto, Verify,
    },
    transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
    ApplyExtrinsicResult,
};

use sp_std::cmp::Ordering;
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        ConstU128, ConstU32, ConstU64, ConstU8, FindAuthor, KeyOwnerProofSystem, OnUnbalanced,
        PrivilegeCmp, Randomness, StorageInfo,
    },
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        IdentityFee, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    },
    ConsensusEngineId, StorageValue,
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{CurrencyAdapter, Multiplier};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

// Frontier
use fp_account::EthereumSignature;
use fp_evm::weight_per_gas;
use fp_rpc::TransactionStatus;
use pallet_ethereum::{Call::transact, PostLogContent, Transaction as EthereumTransaction};
use pallet_evm::{
    Account as EVMAccount, EVMCurrencyAdapter, EnsureAddressNever, EnsureAddressRoot,
    FeeCalculator, IdentityAddressMapping, Runner,
};

mod precompiles;
use precompiles::FrontierPrecompiles;

// Basednode module
pub use pallet_basednode;

// An index to a block.
pub type BlockNumber = u32;

// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = EthereumSignature;

// Some way of identifying an account on the chain. We intentionally make it equivalent
// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

// Balance of an account.
pub type Balance = u128;

// Index of a transaction in the chain.
pub type Index = u32;

// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

// Member type for membership
type MemberCount = u32;

// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
// the specifics of the runtime. They can then be made to be agnostic over specific formats
// of data like extrinsics, allowing for them to continue syncing the network through upgrades
// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    // Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    // Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    // Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("basednode"),
    impl_name: create_runtime_str!("basednode"),
    authoring_version: 1,
    // The version of the runtime specification. A full node will not attempt to use its native
    //   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
    //   `spec_version`, and `authoring_version` are the same between Wasm and native.
    // This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
    //   the compatible custom types.
    spec_version: 141,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 10000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;
    // We allow for 2 seconds of compute with a 6 second average block time.
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::with_sensible_defaults(
            Weight::from_parts(4u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
            NORMAL_DISPATCH_RATIO,
        );
    pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
        ::max_with_normal_ratio(10 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub const SS58Prefix: u16 = 32323;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    // The basic call filter to use in dispatchable.
    type BaseCallFilter = frame_support::traits::Everything;
    // Block & extrinsics weights: base values and limits.
    type BlockWeights = BlockWeights;
    // The maximum length of a block (in bytes).
    type BlockLength = BlockLength;
    // The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    // The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    // The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = sp_runtime::traits::IdentityLookup<AccountId>;
    // The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    // The index type for blocks.
    type BlockNumber = BlockNumber;
    // The type for hashing blocks and tries.
    type Hash = Hash;
    // The hashing algorithm used.
    type Hashing = BlakeTwo256;
    // The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    // The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    // The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    // Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    // The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    // Version of the runtime.
    type Version = Version;
    // Converts a module to the index of the module in `construct_runtime!`.
    //
    // This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    // What to do if a new account is created.
    type OnNewAccount = ();
    // What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    // The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    // Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    // This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    // The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<32>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;

    type KeyOwnerProofSystem = ();

    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type HandleEquivocation = ();

    type WeightInfo = ();
    type MaxAuthorities = ConstU32<32>;
    type MaxSetIdSessionEntries = ConstU64<0>;
}

impl pallet_timestamp::Config for Runtime {
    // A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500_000_000_000;

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    // The type for recording an account's balance.
    type Balance = Balance;
    // The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

impl pallet_evm_chain_id::Config for Runtime {}

// TODO
const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;
pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = WEIGHT_REF_TIME_PER_SECOND.saturating_div(2);

pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            let authority_id = Aura::authorities()[author_index as usize].clone();
            return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
        }
        None
    }
}

// pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);
// impl<R> OnUnbalanced<NegativeImbalance<R>> for ToAuthor<R>
// where
//     R: pallet_balances::Config + pallet_authorship::Config,
//     <R as frame_system::Config>::AccountId: From<AccountId>,
//     <R as frame_system::Config>::AccountId: Into<AccountId>,
//     <R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
// {
//     fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
//         if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
//             <pallet_balances::Pallet<R>>::resolve_creating(&author, amount);
//         }
//     }
// }
// pub struct DealWithFees<R>(sp_std::marker::PhantomData<R>);
// impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithFees<R>
// where
//     R: pallet_balances::Config + pallet_treasury::Config + pallet_authorship::Config,
//     pallet_treasury::Pallet<R>: OnUnbalanced<NegativeImbalance<R>>,
//     <R as frame_system::Config>::AccountId: From<AccountId>,
//     <R as frame_system::Config>::AccountId: Into<AccountId>,
//     <R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
// {
//     fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<R>>) {
//         if let Some(fees) = fees_then_tips.next() {
//             // for fees, 75% to treasury, 25% to author
//             let mut split = fees.ration(75, 25);
//             if let Some(tips) = fees_then_tips.next() {
//                 // for tips, if any, 100% to author
//                 tips.merge_into(&mut split.1);
//             }
//             use pallet_treasury::Pallet as Treasury;
//             <Treasury<R> as OnUnbalanced<_>>::on_unbalanced(split.0);
//             <ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(split.1);
//         }
//     }
// }

// TODO
const BLOCK_GAS_LIMIT: u64 = 750_000_000_000_000_000; // 750*10^15

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(BLOCK_GAS_LIMIT);
    pub const GasLimitPovSizeRatio: u64 = BLOCK_GAS_LIMIT.saturating_div(MAX_POV_SIZE);
    pub PrecompilesValue: FrontierPrecompiles<Runtime> =
        FrontierPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_ref_time(weight_per_gas(BLOCK_GAS_LIMIT,
            NORMAL_DISPATCH_RATIO, WEIGHT_MILLISECS_PER_BLOCK));
    pub SuicideQuickClearLimit: u32 = 0;
}

impl pallet_evm::Config for Runtime {
    // type FeeCalculator = pallet_dynamic_fee::Module<Self>;
    type FeeCalculator = BaseFee;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;

    #[doc = r" Weight corresponding to a gas unit."]
    type WeightPerGas = WeightPerGas;

    #[doc = r" Block number to block hash."]
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;

    #[doc = r" Allow the origin to call on behalf of given address."]
    type CallOrigin = EnsureAddressRoot<AccountId>;

    #[doc = r" Allow the origin to withdraw on behalf of given address."]
    type WithdrawOrigin = EnsureAddressNever<AccountId>;

    #[doc = r" Mapping from address to account id."]
    type AddressMapping = IdentityAddressMapping;

    #[doc = r" Currency type for withdraw and balance storage."]
    type Currency = Balances;

    #[doc = r" The overarching event type."]
    type RuntimeEvent = RuntimeEvent;

    #[doc = r" Precompiles associated with this EVM engine."]
    type PrecompilesType = FrontierPrecompiles<Self>;

    type PrecompilesValue = PrecompilesValue;

    #[doc = r" Chain ID of EVM."]
    type ChainId = EVMChainId;

    #[doc = r" The block gas limit. Can be a simple constant, or an adjustment algorithm in another pallet."]
    type BlockGasLimit = BlockGasLimit;

    #[doc = r" EVM execution runner."]
    type Runner = pallet_evm::runner::stack::Runner<Self>;

    #[doc = r" To handle fee deduction for EVM transactions. An example is this pallet being used by `pallet_ethereum`"]
    #[doc = r" where the chain implementing `pallet_ethereum` should be able to configure what happens to the fees"]
    #[doc = r" Similar to `OnChargeTransaction` of `pallet_transaction_payment`"]
    type OnChargeTransaction = EVMCurrencyAdapter<Balances, ()>;
    // type OnChargeTransaction = ();

    #[doc = r" Called on create calls, used to record owner"]
    type OnCreate = ();

    #[doc = r" Find author for the current block."]
    type FindAuthor = FindAuthorTruncated<Aura>;
}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
}

parameter_types! {
    pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
    type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
    pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill {
        Permill::zero()
    }
    fn ideal() -> Permill {
        Permill::from_parts(500_000)
    }
    fn upper() -> Permill {
        Permill::from_parts(1_000_000)
    }
}

impl pallet_base_fee::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Threshold = BaseFeeThreshold;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type DefaultElasticity = DefaultElasticity;
}

impl pallet_hotfix_sufficients::Config for Runtime {
    type AddressMapping = IdentityAddressMapping;
    type WeightInfo = pallet_hotfix_sufficients::weights::SubstrateWeight<Runtime>;
}

pub struct LinearWeightToFee<C>(sp_std::marker::PhantomData<C>);

impl<C> WeightToFeePolynomial for LinearWeightToFee<C>
where
    C: Get<Balance>,
{
    type Balance = Balance;

    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        let coefficient = WeightToFeeCoefficient {
            coeff_integer: 0,
            coeff_frac: Perbill::from_parts(1),
            negative: false,
            degree: 1,
        };

        smallvec!(coefficient)
    }
}

parameter_types! {
    // Used with LinearWeightToFee conversion.
    pub const FeeWeightRatio: u64 = 1;
    pub const TransactionByteFee: u128 = 1;
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;

    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    //type TransactionByteFee = TransactionByteFee;

    // Convert dispatch weight to a chargeable fee.
    type WeightToFee = LinearWeightToFee<FeeWeightRatio>;

    type FeeMultiplierUpdate = ();

    type OperationalFeeMultiplier = ConstU8<1>;

    type LengthToFee = IdentityFee<Balance>;
    //type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

// Configure collective pallet for council
parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 12 * HOURS;
    pub const CouncilMaxProposals: u32 = 10;
    pub const CouncilMaxMembers: u32 = 3;
}

// Configure collective pallet for Senate
parameter_types! {
    pub const SenateMaxMembers: u32 = 12;
}

use pallet_collective::{CanPropose, CanVote, GetVotingMembers};
pub struct CanProposeToTriumvirate;
impl CanPropose<AccountId> for CanProposeToTriumvirate {
    fn can_propose(account: &AccountId) -> bool {
        Triumvirate::is_member(account)
    }
}

pub struct CanVoteToTriumvirate;
impl CanVote<AccountId> for CanVoteToTriumvirate {
    fn can_vote(_: &AccountId) -> bool {
        //Senate::is_member(account)
        false // Disable voting from pallet_collective::vote
    }
}

use pallet_basednode::{CollectiveInterface, MemberManagement};
pub struct ManageSenateMembers;
impl MemberManagement<AccountId> for ManageSenateMembers {
    fn add_member(account: &AccountId) -> DispatchResult {
        SenateMembers::add_member(RawOrigin::Root.into(), account.clone())
    }

    fn remove_member(account: &AccountId) -> DispatchResult {
        SenateMembers::remove_member(RawOrigin::Root.into(), account.clone())
    }

    fn swap_member(rm: &AccountId, add: &AccountId) -> DispatchResult {
        Triumvirate::remove_votes(rm)?;
        SenateMembers::swap_member(RawOrigin::Root.into(), rm.clone(), add.clone())
    }

    fn is_member(account: &AccountId) -> bool {
        SenateMembers::members().contains(account)
    }

    fn members() -> Vec<AccountId> {
        SenateMembers::members().into()
    }

    fn max_members() -> u32 {
        SenateMaxMembers::get()
    }
}

pub struct GetSenateMemberCount;
impl GetVotingMembers<MemberCount> for GetSenateMemberCount {
    fn get_count() -> MemberCount {
        SenateMembers::members().len() as u32
    }
}
impl Get<MemberCount> for GetSenateMemberCount {
    fn get() -> MemberCount {
        SenateMaxMembers::get()
    }
}

pub struct TriumvirateVotes;
impl CollectiveInterface<AccountId, Hash, u32> for TriumvirateVotes {
    fn remove_votes(computekey: &AccountId) -> Result<bool, sp_runtime::DispatchError> {
        Triumvirate::remove_votes(computekey)
    }

    fn add_vote(
        computekey: &AccountId,
        proposal: Hash,
        index: u32,
        approve: bool,
    ) -> Result<bool, sp_runtime::DispatchError> {
        Triumvirate::do_vote(computekey.clone(), proposal, index, approve)
    }
}

type EnsureMajoritySenate =
    pallet_collective::EnsureProportionMoreThan<AccountId, TriumvirateCollective, 1, 2>;

// We call pallet_collective TriumvirateCollective
type TriumvirateCollective = pallet_collective::Instance1;
impl pallet_collective::Config<TriumvirateCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = GetSenateMemberCount;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureNever<AccountId>;
    type CanPropose = CanProposeToTriumvirate;
    type CanVote = CanVoteToTriumvirate;
    type GetVotingMembers = GetSenateMemberCount;
}

// We call council members Triumvirate
type TriumvirateMembership = pallet_membership::Instance1;
impl pallet_membership::Config<TriumvirateMembership> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRoot<AccountId>;
    type RemoveOrigin = EnsureRoot<AccountId>;
    type SwapOrigin = EnsureRoot<AccountId>;
    type ResetOrigin = EnsureRoot<AccountId>;
    type PrimeOrigin = EnsureRoot<AccountId>;
    type MembershipInitialized = Triumvirate;
    type MembershipChanged = Triumvirate;
    type MaxMembers = CouncilMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

// We call our top K delegates membership Senate
type SenateMembership = pallet_membership::Instance2;
impl pallet_membership::Config<SenateMembership> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = EnsureRoot<AccountId>;
    type RemoveOrigin = EnsureRoot<AccountId>;
    type SwapOrigin = EnsureRoot<AccountId>;
    type ResetOrigin = EnsureRoot<AccountId>;
    type PrimeOrigin = EnsureRoot<AccountId>;
    type MembershipInitialized = ();
    type MembershipChanged = ();
    type MaxMembers = SenateMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = (1) as Balance * 2_000 * 10_000 + (88 as Balance) * 100 * 10_000;
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = (0) as Balance * 2_000 * 10_000 + (32 as Balance) * 100 * 10_000;
    pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        BlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
    pub const NoPreimagePostponement: Option<u32> = Some(10);
}

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
    fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
        if left == right {
            return Some(Ordering::Equal);
        }

        match (left, right) {
            // Root is greater than anything.
            (OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
            // Check which one has more yes votes.
            (
                OriginCaller::Triumvirate(pallet_collective::RawOrigin::Members(
                    l_yes_votes,
                    l_count,
                )),
                OriginCaller::Triumvirate(pallet_collective::RawOrigin::Members(
                    r_yes_votes,
                    r_count,
                )), // Equivalent to (l_yes_votes / l_count).cmp(&(r_yes_votes / r_count))
            ) => Some((l_yes_votes * r_count).cmp(&(r_yes_votes * l_count))),
            // For every other origin we don't care, as they are not used for `ScheduleOrigin`.
            _ => None,
        }
    }
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeEvent = RuntimeEvent;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
    type OriginPrivilegeCmp = OriginPrivilegeCmp;
    type Preimages = Preimage;
}

parameter_types! {
    pub const PreimageMaxSize: u32 = 4096 * 1024;
    pub const PreimageBaseDeposit: Balance = (2) as Balance * 2_000 * 10_000_000 + (64 as Balance) * 100 * 1_000_000;
    pub const PreimageByteDeposit: Balance = (0) as Balance * 2_000 * 10_000_000 + (1 as Balance) * 100 * 1_000_000;
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type BaseDeposit = PreimageBaseDeposit;
    type ByteDeposit = PreimageByteDeposit;
}

pub struct AllowIdentityReg;

impl CanRegisterIdentity<AccountId> for AllowIdentityReg {
    #[cfg(not(feature = "runtime-benchmarks"))]
    fn can_register(address: &AccountId, identified: &AccountId) -> bool {
        if address != identified {
            return BasedNode::personalkey_owns_computekey(address, identified)
                && BasedNode::is_computekey_registered_on_network(0, identified);
        } else {
            return BasedNode::is_brain_owner(address);
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn can_register(_: &AccountId, _: &AccountId) -> bool {
        true
    }
}

// Configure registry pallet.
parameter_types! {
    pub const MaxAdditionalFields: u32 = 1;
    pub const InitialDeposit: Balance = 100_000_000_000_000_000; // 0.1 BASED
    pub const FieldDeposit: Balance = 100_000_000_000_000_000; // 0.1 BASED
}

impl pallet_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type CanRegister = AllowIdentityReg;
    type WeightInfo = pallet_registry::weights::SubstrateWeight<Runtime>;

    type MaxAdditionalFields = MaxAdditionalFields;
    type InitialDeposit = InitialDeposit;
    type FieldDeposit = FieldDeposit;
}

parameter_types! {
    pub const MaxCommitFields: u32 = 1;
    pub const CommitmentInitialDeposit: Balance = 0; // Free
    pub const CommitmentFieldDeposit: Balance = 0; // Free
    pub const CommitmentRateLimit: BlockNumber = 100; // Allow commitment every 100 blocks
}

pub struct AllowCommitments;
impl CanCommit<AccountId> for AllowCommitments {
    #[cfg(not(feature = "runtime-benchmarks"))]
    fn can_commit(netuid: u16, address: &AccountId) -> bool {
        BasedNode::is_computekey_registered_on_network(netuid, address)
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn can_commit(_: u16, _: &AccountId) -> bool {
        true
    }
}

impl pallet_commitments::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type WeightInfo = pallet_commitments::weights::SubstrateWeight<Runtime>;

    type CanCommit = AllowCommitments;

    type MaxFields = MaxCommitFields;
    type InitialDeposit = CommitmentInitialDeposit;
    type FieldDeposit = CommitmentFieldDeposit;
    type RateLimit = CommitmentRateLimit;
}

// Configure the pallet basednode.
parameter_types! {
    pub const BasednodeInitialRho: u16 = 10;
    pub const BasednodeInitialKappa: u16 = 32_767; // 0.5 = 65535/2
    pub const BasednodeInitialMaxAllowedUids: u16 = 2048;
    pub const BasednodeInitialIssuance: u128 = 0;
    pub const BasednodeInitialMinAllowedWeights: u16 = 1024;
    pub const BasednodeInitialEmissionValue: u16 = 0;
    pub const BasednodeInitialMaxWeightsLimit: u16 = 1000; // 1000/2^16 = 0.015
    pub const BasednodeInitialValidatorPruneLen: u64 = 1;
    pub const BasednodeInitialScalingLawPower: u16 = 50; // 0.5
    pub const BasednodeInitialMaxAllowedValidators: u16 = 256;
    pub const BasednodeInitialTempo: u16 = 99;
    pub const BasednodeInitialDifficulty: u64 = 10_000_000;
    pub const BasednodeInitialAdjustmentInterval: u16 = 100;
    pub const BasednodeInitialAdjustmentAlpha: u64 = 0; // no weight to previous value.
    pub const BasednodeInitialTargetRegistrationsPerInterval: u16 = 2;
    pub const BasednodeInitialImmunityPeriod: u16 = 4096;
    pub const BasednodeInitialActivityCutoff: u16 = 5000;
    pub const BasednodeInitialMaxRegistrationsPerBlock: u16 = 1;
    pub const BasednodeInitialPruningScore : u16 = u16::MAX;
    pub const BasednodeInitialBondsMovingAverage: u64 = 900_000;
    pub const BasednodeInitialDefaultTake: u16 = 26_214; // 40% honest number.
    pub const BasednodeInitialWeightsVersionKey: u64 = 0;
    pub const BasednodeInitialMinDifficulty: u64 = 10_000_000;
    pub const BasednodeInitialMaxDifficulty: u64 = u64::MAX / 4;
    pub const BasednodeInitialServingRateLimit: u64 = 50;
    pub const BasednodeInitialBurn: u128 = 1_000_000_000_000_000_000_000; // 1000 based
    pub const BasednodeInitialMinBurn: u128 = 1_000_000_000_000_000_000_000; // 1000 based
    pub const BasednodeInitialMaxBurn: u128 = 100_000_000_000_000_000_000_000; // 100_000 based
    pub const BasednodeInitialTxRateLimit: u64 = 1000;
    pub const BasednodeInitialRAORecycledForRegistration: u128 = 0; // 0 rao
    pub const BasednodeInitialSenateRequiredStakePercentage: u64 = 1; // 1 percent of total stake
    pub const BasednodeInitialNetworkImmunity: u64 = 7 * 7200;
    pub const BasednodeInitialMinAllowedUids: u16 = 128;
    pub const BasednodeInitialMinLockCost: u128 = 1_000_000_000_000_000_000_000; // 1000 BASED
    pub const BasednodeInitialBrainOwnerCut: u16 = 26_214; // 40 percent, gets subdivided between ERC token holders in block_step
    pub const BasednodeInitialBrainLimit: u16 = 1_024;
    pub const BasednodeInitialNetworkLockReductionInterval: u64 = 14 * 7200;
    pub const BasednodeInitialNetworkRateLimit: u64 = 1 * 7200;
	pub const BasednodeInitialBrainOwnerByTokenCut: u128 = 26_214;
}

impl pallet_basednode::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SudoRuntimeCall = RuntimeCall;
    type Currency = Balances;
    type CouncilOrigin = EnsureMajoritySenate;
    type SenateMembers = ManageSenateMembers;
    type TriumvirateInterface = TriumvirateVotes;

    type InitialRho = BasednodeInitialRho;
    type InitialKappa = BasednodeInitialKappa;
    type InitialMaxAllowedUids = BasednodeInitialMaxAllowedUids;
    type InitialBondsMovingAverage = BasednodeInitialBondsMovingAverage;
    type InitialIssuance = BasednodeInitialIssuance;
    type InitialMinAllowedWeights = BasednodeInitialMinAllowedWeights;
    type InitialEmissionValue = BasednodeInitialEmissionValue;
    type InitialMaxWeightsLimit = BasednodeInitialMaxWeightsLimit;
    type InitialValidatorPruneLen = BasednodeInitialValidatorPruneLen;
    type InitialScalingLawPower = BasednodeInitialScalingLawPower;
    type InitialTempo = BasednodeInitialTempo;
    type InitialDifficulty = BasednodeInitialDifficulty;
    type InitialAdjustmentInterval = BasednodeInitialAdjustmentInterval;
    type InitialAdjustmentAlpha = BasednodeInitialAdjustmentAlpha;
    type InitialTargetRegistrationsPerInterval = BasednodeInitialTargetRegistrationsPerInterval;
    type InitialImmunityPeriod = BasednodeInitialImmunityPeriod;
    type InitialActivityCutoff = BasednodeInitialActivityCutoff;
    type InitialMaxRegistrationsPerBlock = BasednodeInitialMaxRegistrationsPerBlock;
    type InitialPruningScore = BasednodeInitialPruningScore;
    type InitialMaxAllowedValidators = BasednodeInitialMaxAllowedValidators;
    type InitialDefaultTake = BasednodeInitialDefaultTake;
    type InitialWeightsVersionKey = BasednodeInitialWeightsVersionKey;
    type InitialMaxDifficulty = BasednodeInitialMaxDifficulty;
    type InitialMinDifficulty = BasednodeInitialMinDifficulty;
    type InitialServingRateLimit = BasednodeInitialServingRateLimit;
    type InitialBurn = BasednodeInitialBurn;
    type InitialMaxBurn = BasednodeInitialMaxBurn;
    type InitialMinBurn = BasednodeInitialMinBurn;
    type InitialTxRateLimit = BasednodeInitialTxRateLimit;
    type InitialRAORecycledForRegistration = BasednodeInitialRAORecycledForRegistration;
    type InitialSenateRequiredStakePercentage = BasednodeInitialSenateRequiredStakePercentage;
    type InitialNetworkImmunityPeriod = BasednodeInitialNetworkImmunity;
    type InitialNetworkMinAllowedUids = BasednodeInitialMinAllowedUids;
    type InitialNetworkMinLockCost = BasednodeInitialMinLockCost;
    type InitialNetworkLockReductionInterval = BasednodeInitialNetworkLockReductionInterval;
    type InitialBrainOwnerCut = BasednodeInitialBrainOwnerCut;
    type InitialBrainLimit = BasednodeInitialBrainLimit;
    type InitialNetworkRateLimit = BasednodeInitialNetworkRateLimit;
    type InitialBrainOwnerByTokenBalanceCut = BasednodeInitialBrainOwnerByTokenCut;
}

use sp_runtime::BoundedVec;

pub struct AuraPalletIntrf;
impl pallet_admin_utils::AuraInterface<AuraId, ConstU32<32>> for AuraPalletIntrf {
    fn change_authorities(new: BoundedVec<AuraId, ConstU32<32>>) {
        Aura::change_authorities(new);
    }
}

pub struct BasednodeInterface;

impl
    pallet_admin_utils::BasednodeInterface<
        AccountId,
        <pallet_balances::Pallet<Runtime> as frame_support::traits::Currency<AccountId>>::Balance,
        RuntimeOrigin,
    > for BasednodeInterface
{
    fn set_default_take(default_take: u16) {
        BasedNode::set_default_take(default_take);
    }

    fn set_tx_rate_limit(rate_limit: u64) {
        BasedNode::set_tx_rate_limit(rate_limit);
    }

    fn set_serving_rate_limit(netuid: u16, rate_limit: u64) {
        BasedNode::set_serving_rate_limit(netuid, rate_limit);
    }

    fn set_max_burn(netuid: u16, max_burn: u128) {
        BasedNode::set_max_burn(netuid, max_burn);
    }

    fn set_min_burn(netuid: u16, min_burn: u128) {
        BasedNode::set_min_burn(netuid, min_burn);
    }

    fn set_burn(netuid: u16, burn: u128) {
        BasedNode::set_burn(netuid, burn);
    }

    fn set_max_difficulty(netuid: u16, max_diff: u64) {
        BasedNode::set_max_difficulty(netuid, max_diff);
    }

    fn set_min_difficulty(netuid: u16, min_diff: u64) {
        BasedNode::set_min_difficulty(netuid, min_diff);
    }

    fn set_difficulty(netuid: u16, diff: u64) {
        BasedNode::set_difficulty(netuid, diff);
    }

    fn set_weights_rate_limit(netuid: u16, rate_limit: u64) {
        BasedNode::set_weights_set_rate_limit(netuid, rate_limit);
    }

    fn set_weights_version_key(netuid: u16, version: u64) {
        BasedNode::set_weights_version_key(netuid, version);
    }

    fn set_bonds_moving_average(netuid: u16, moving_average: u64) {
        BasedNode::set_bonds_moving_average(netuid, moving_average);
    }

    fn set_max_allowed_validators(netuid: u16, max_validators: u16) {
        BasedNode::set_max_allowed_validators(netuid, max_validators);
    }

    fn get_root_netuid() -> u16 {
        return BasedNode::get_root_netuid();
    }

    fn if_brain_exist(netuid: u16) -> bool {
        return BasedNode::if_brain_exist(netuid);
    }

    fn create_account_if_non_existent(personalkey: &AccountId, computekey: &AccountId) {
        return BasedNode::create_account_if_non_existent(personalkey, computekey);
    }

    fn personalkey_owns_computekey(personalkey: &AccountId, computekey: &AccountId) -> bool {
        return BasedNode::personalkey_owns_computekey(personalkey, computekey);
    }

    fn increase_stake_on_personalkey_computekey_account(
        personalkey: &AccountId,
        computekey: &AccountId,
        increment: u64,
    ) {
        BasedNode::increase_stake_on_personalkey_computekey_account(
            personalkey,
            computekey,
            increment,
        );
    }

    fn u64_to_balance(input: u64) -> Option<Balance> {
        return BasedNode::u64_to_balance(input);
    }

    fn add_balance_to_personalkey_account(personalkey: &AccountId, amount: Balance) {
        BasedNode::add_balance_to_personalkey_account(personalkey, amount);
    }

    fn get_current_block_as_u64() -> u64 {
        return BasedNode::get_current_block_as_u64();
    }

    fn get_brain_n(netuid: u16) -> u16 {
        return BasedNode::get_brain_n(netuid);
    }

    fn get_max_allowed_uids(netuid: u16) -> u16 {
        return BasedNode::get_max_allowed_uids(netuid);
    }

    fn append_agent(netuid: u16, new_computekey: &AccountId, block_number: u64) {
        return BasedNode::append_agent(netuid, new_computekey, block_number);
    }

    fn get_agent_to_prune(netuid: u16) -> u16 {
        return BasedNode::get_agent_to_prune(netuid);
    }

    fn replace_agent(
        netuid: u16,
        uid_to_replace: u16,
        new_computekey: &AccountId,
        block_number: u64,
    ) {
        BasedNode::replace_agent(netuid, uid_to_replace, new_computekey, block_number);
    }

    fn set_total_issuance(total_issuance: u128) {
        BasedNode::set_total_issuance(total_issuance);
    }

    fn set_network_immunity_period(net_immunity_period: u64) {
        BasedNode::set_network_immunity_period(net_immunity_period);
    }

    fn set_network_min_lock(net_min_lock: u128) {
        BasedNode::set_network_min_lock(net_min_lock);
    }

    fn set_brain_limit(limit: u16) {
        BasedNode::set_max_brains(limit);
    }

    fn set_lock_reduction_interval(interval: u64) {
        BasedNode::set_lock_reduction_interval(interval);
    }

    fn set_tempo(netuid: u16, tempo: u16) {
        BasedNode::set_tempo(netuid, tempo);
    }

    fn set_brain_owner_cut(brain_owner_cut: u16) {
        BasedNode::set_brain_owner_cut(brain_owner_cut);
    }

    fn set_network_rate_limit(limit: u64) {
        BasedNode::set_network_rate_limit(limit);
    }

    fn set_max_registrations_per_block(netuid: u16, max_registrations_per_block: u16) {
        BasedNode::set_max_registrations_per_block(netuid, max_registrations_per_block);
    }

    fn set_adjustment_alpha(netuid: u16, adjustment_alpha: u64) {
        BasedNode::set_adjustment_alpha(netuid, adjustment_alpha);
    }

    fn set_target_registrations_per_interval(netuid: u16, target_registrations_per_interval: u16) {
        BasedNode::set_target_registrations_per_interval(netuid, target_registrations_per_interval);
    }

    fn set_network_pow_registration_allowed(netuid: u16, registration_allowed: bool) {
        BasedNode::set_network_pow_registration_allowed(netuid, registration_allowed);
    }

    fn set_network_registration_allowed(netuid: u16, registration_allowed: bool) {
        BasedNode::set_network_registration_allowed(netuid, registration_allowed);
    }

    fn set_activity_cutoff(netuid: u16, activity_cutoff: u16) {
        BasedNode::set_activity_cutoff(netuid, activity_cutoff);
    }

    fn ensure_brain_owner_or_root(o: RuntimeOrigin, netuid: u16) -> Result<(), DispatchError> {
        return BasedNode::ensure_brain_owner_or_root(o, netuid);
    }

    fn set_rho(netuid: u16, rho: u16) {
        BasedNode::set_rho(netuid, rho);
    }

    fn set_kappa(netuid: u16, kappa: u16) {
        BasedNode::set_kappa(netuid, kappa);
    }

    fn set_max_allowed_uids(netuid: u16, max_allowed: u16) {
        BasedNode::set_max_allowed_uids(netuid, max_allowed);
    }

    fn set_min_allowed_weights(netuid: u16, min_allowed_weights: u16) {
        BasedNode::set_min_allowed_weights(netuid, min_allowed_weights);
    }

    fn set_immunity_period(netuid: u16, immunity_period: u16) {
        BasedNode::set_immunity_period(netuid, immunity_period);
    }

    fn set_max_weight_limit(netuid: u16, max_weight_limit: u16) {
        BasedNode::set_max_weight_limit(netuid, max_weight_limit);
    }

    fn set_scaling_law_power(netuid: u16, scaling_law_power: u16) {
        BasedNode::set_scaling_law_power(netuid, scaling_law_power);
    }

    fn set_validator_prune_len(netuid: u16, validator_prune_len: u64) {
        BasedNode::set_validator_prune_len(netuid, validator_prune_len);
    }

    fn set_adjustment_interval(netuid: u16, adjustment_interval: u16) {
        BasedNode::set_adjustment_interval(netuid, adjustment_interval);
    }

    fn set_weights_set_rate_limit(netuid: u16, weights_set_rate_limit: u64) {
        BasedNode::set_weights_set_rate_limit(netuid, weights_set_rate_limit);
    }

    fn set_rao_recycled(netuid: u16, rao_recycled: u128) {
        BasedNode::set_rao_recycled(netuid, rao_recycled);
    }

    fn is_computekey_registered_on_network(netuid: u16, computekey: &AccountId) -> bool {
        return BasedNode::is_computekey_registered_on_network(netuid, computekey);
    }

    fn init_new_network(netuid: u16, tempo: u16) {
        BasedNode::init_new_network(netuid, tempo);
    }
}

impl pallet_admin_utils::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AuthorityId = AuraId;
    type MaxAuthorities = ConstU32<32>;
    type Aura = AuraPalletIntrf;
    type Balance = Balance;
    type Basednode = BasednodeInterface;
    type WeightInfo = pallet_admin_utils::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub struct Runtime
    where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        BasedNode: pallet_basednode,
        Triumvirate: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
        TriumvirateMembers: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
        SenateMembers: pallet_membership::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},
        Utility: pallet_utility,
        Sudo: pallet_sudo,
        Multisig: pallet_multisig,
        Preimage: pallet_preimage,
        Scheduler: pallet_scheduler,
        Registry: pallet_registry,
        Commitments: pallet_commitments,
        AdminUtils: pallet_admin_utils,
        Ethereum: pallet_ethereum,
        EVM: pallet_evm,
        EVMChainId: pallet_evm_chain_id,
        DynamicFee: pallet_dynamic_fee,
        BaseFee: pallet_base_fee,
        HotfixSufficiens: pallet_hotfix_sufficients,
    }
);

#[derive(Clone)]
pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
        UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        )
    }
}

impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
    fn convert_transaction(
        &self,
        transaction: pallet_ethereum::Transaction,
    ) -> opaque::UncheckedExtrinsic {
        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        );
        let encoded = extrinsic.encode();
        opaque::UncheckedExtrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => {
                call.pre_dispatch_self_contained(info, dispatch_info, len)
            }
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
                Some(call.dispatch(RuntimeOrigin::from(
                    pallet_ethereum::RawOrigin::EthereumTransaction(info),
                )))
            }
            _ => None,
        }
    }
}

// The address format for describing accounts.
pub type Address = AccountId;
// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    pallet_basednode::BasednodeSignedExtension<Runtime>,
    pallet_commitments::CommitmentsSignedExtension<Runtime>,
);

// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    define_benchmarks!(
        [frame_benchmarking, BaselineBench::<Runtime>]
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_basednode, BasedNode]
        [pallet_timestamp, Timestamp]
        [pallet_registry, Registry]
        [pallet_commitments, Commitments]
        [pallet_admin_utils, AdminUtils]
    );
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            _authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            // NOTE: this is the only implementation possible since we've
            // defined our key owner proof type as a bottom type (i.e. a type
            // with no values).
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            use frame_support::traits::WhitelistedStorageKeys;
            let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            Ok(batches)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here. If any of the pre/post migration checks fail, we shall stop
            // right here and right now.
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, BlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
        }
    }

    impl basednode_custom_rpc_runtime_api::DelegateInfoRuntimeApi<Block> for Runtime {
        fn get_delegates() -> Vec<u8> {
            let result = BasedNode::get_delegates();
            result.encode()
        }

        fn get_delegate(delegate_account_vec: Vec<u8>) -> Vec<u8> {
            let _result = BasedNode::get_delegate(delegate_account_vec);
            if _result.is_some() {
                let result = _result.expect("Could not get DelegateInfo");
                result.encode()
            } else {
                vec![]
            }
        }

        fn get_delegated(delegatee_account_vec: Vec<u8>) -> Vec<u8> {
            let result = BasedNode::get_delegated(delegatee_account_vec);
            result.encode()
        }
    }

    impl basednode_custom_rpc_runtime_api::AgentInfoRuntimeApi<Block> for Runtime {
        fn get_agents_lite(netuid: u16) -> Vec<u8> {
            let result = BasedNode::get_agents_lite(netuid);
            result.encode()
        }

        fn get_agent_lite(netuid: u16, uid: u16) -> Vec<u8> {
            let _result = BasedNode::get_agent_lite(netuid, uid);
            if _result.is_some() {
                let result = _result.expect("Could not get AgentInfoLite");
                result.encode()
            } else {
                vec![]
            }
        }

        fn get_agents(netuid: u16) -> Vec<u8> {
            let result = BasedNode::get_agents(netuid);
            result.encode()
        }

        fn get_agent(netuid: u16, uid: u16) -> Vec<u8> {
            let _result = BasedNode::get_agent(netuid, uid);
            if _result.is_some() {
                let result = _result.expect("Could not get AgentInfo");
                result.encode()
            } else {
                vec![]
            }
        }
    }

    impl basednode_custom_rpc_runtime_api::BrainInfoRuntimeApi<Block> for Runtime {
        fn get_brain_info(netuid: u16) -> Vec<u8> {
            let _result = BasedNode::get_brain_info(netuid);
            if _result.is_some() {
                let result = _result.expect("Could not get BrainInfo");
                result.encode()
            } else {
                vec![]
            }
        }

        fn get_brains_info() -> Vec<u8> {
            let result = BasedNode::get_brains_info();
            result.encode()
        }

        fn get_brain_hyperparams(netuid: u16) -> Vec<u8> {
            let _result = BasedNode::get_brain_hyperparams(netuid);
            if _result.is_some() {
                let result = _result.expect("Could not get BrainHyperparams");
                result.encode()
            } else {
                vec![]
            }
        }
    }

    impl basednode_custom_rpc_runtime_api::StakeInfoRuntimeApi<Block> for Runtime {
        fn get_stake_info_for_personalkey( personalkey_account_vec: Vec<u8> ) -> Vec<u8> {
            let result = BasedNode::get_stake_info_for_personalkey( personalkey_account_vec );
            result.encode()
        }

        fn get_stake_info_for_personalkeys( personalkey_account_vecs: Vec<Vec<u8>> ) -> Vec<u8> {
            let result = BasedNode::get_stake_info_for_personalkeys( personalkey_account_vecs );
            result.encode()
        }
    }

    impl basednode_custom_rpc_runtime_api::BrainRegistrationRuntimeApi<Block> for Runtime {
        fn get_network_registration_cost() -> u128 {
            BasedNode::get_network_lock_cost()
        }
    }

    impl basednode_custom_rpc_runtime_api::TftEnforcerDataRuntimeApi<Block> for Runtime {
        fn get_tft_enforcer_data(from_block: Vec<u8>, block_count: Option<u64>) -> Vec<u8> {
            let result = BasedNode::get_tft_enforcer_data(from_block, block_count);
            result.encode()
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            <Runtime as pallet_evm::Config>::ChainId::get()
        }

        fn account_basic(address: H160) -> EVMAccount {
            let (account, _) = EVM::account_basic(&address);
            account
        }

        fn gas_price() -> U256 {
            let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
            gas_price
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            EVM::account_codes(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            EVM::account_storages(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let is_transactional = false;
            let validate = true;
            let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());
            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                validate,
                evm_config,
            ).map_err(|err| err.error.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let is_transactional = false;
            let validate = true;
            let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());
            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.unique_saturated_into(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                validate,
                evm_config,
            ).map_err(|err| err.error.into())
        }

        fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
            Ethereum::current_transaction_statuses()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            Ethereum::current_block()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            Ethereum::current_receipts()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<TransactionStatus>>
        ) {
            (
                Ethereum::current_block(),
                Ethereum::current_receipts(),
                Ethereum::current_transaction_statuses()
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<EthereumTransaction> {
            xts.into_iter().filter_map(|xt| match xt.0.function {
                RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
                _ => None
            }).collect::<Vec<EthereumTransaction>>()
        }

        fn elasticity() -> Option<Permill> {
            Some(BaseFee::elasticity())
        }

        fn gas_limit_multiplier_support() {}
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(transaction: EthereumTransaction) -> <Block as
            BlockT>::Extrinsic {
                UncheckedExtrinsic::new_unsigned(
                    pallet_ethereum::Call::<Runtime>::transact
                    { transaction }.into(),
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::traits::WhitelistedStorageKeys;
    use sp_core::hexdisplay::HexDisplay;
    use std::collections::HashSet;

    #[test]
    fn check_whitelist() {
        let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
            .iter()
            .map(|e| HexDisplay::from(&e.key).to_string())
            .collect();

        // Block Number
        assert!(
            whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
        );
        // Total Issuance
        assert!(
            whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
        );
        // Execution Phase
        assert!(
            whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
        );
        // Event Count
        assert!(
            whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
        );
        // System Events
        assert!(
            whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
        );
    }
}
