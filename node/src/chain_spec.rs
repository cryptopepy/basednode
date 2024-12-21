use basednode_runtime::{
    AccountId, AuraConfig, BalancesConfig, BasedNodeConfig, EVMConfig, GenesisConfig,
    GrandpaConfig, SS58Prefix, SenateMembersConfig, Signature, SudoConfig, SystemConfig,
    TriumvirateConfig, TriumvirateMembersConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::crypto::Ss58Codec;
use sp_core::{bounded_vec, sr25519, Pair, Public, H160, U256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::env;
use std::{collections::BTreeMap, str::FromStr};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

// These functions are unused in production compiles, util functions for unit testing
#[allow(dead_code)]
/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

#[allow(dead_code)]
type AccountPublic = <Signature as Verify>::Signer;

#[allow(dead_code)]
/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

#[allow(dead_code)]
/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn authority_keys_from_ss58(s_aura: &str, s_grandpa: &str) -> (AuraId, GrandpaId) {
    (
        get_aura_from_ss58_addr(s_aura),
        get_grandpa_from_ss58_addr(s_grandpa),
    )
}

pub fn get_aura_from_ss58_addr(s: &str) -> AuraId {
    Ss58Codec::from_ss58check(s).unwrap()
}

pub fn get_grandpa_from_ss58_addr(s: &str) -> GrandpaId {
    Ss58Codec::from_ss58check(s).unwrap()
}

// Includes for nakamoto genesis
use serde::Deserialize;
use serde_json as json;
use std::{fs::File, path::PathBuf};

// Configure storage from nakamoto data
#[derive(Deserialize, Debug)]
struct PersonalkeyComputekeys {
    stakes: std::collections::HashMap<String, std::collections::HashMap<String, (u128, u16)>>,
    balances: std::collections::HashMap<String, String>, // address, amount
}

pub fn prometheus_mainnet_config() -> Result<ChainSpec, String> {
    let path: PathBuf = std::path::PathBuf::from("./snapshot.json");
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    // We mmap the file into memory first, as this is *a lot* faster than using
    // `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
    let file = File::open(&path)
        .map_err(|e| format!("Error opening genesis file `{}`: {}", path.display(), e))?;

    // SAFETY: `mmap` is fundamentally unsafe since technically the file can change
    //         underneath us while it is mapped; in practice it's unlikely to be a problem
    let bytes = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Error mmaping genesis file `{}`: {}", path.display(), e))?
    };

    let old_state: PersonalkeyComputekeys =
        json::from_slice(&bytes).map_err(|e| format!("Error parsing genesis file: {}", e))?;

    let mut processed_stakes: Vec<(AccountId, Vec<(AccountId, (u128, u16))>)> = Vec::new();
    for (personalkey_str, computekeys) in old_state.stakes.iter() {
        let personalkey_account = AccountId::from_str(&personalkey_str).unwrap();

        let mut processed_computekeys: Vec<(AccountId, (u128, u16))> = Vec::new();

        for (computekey_str, amount_uid) in computekeys.iter() {
            let (amount, uid) = amount_uid;
            let computekey_account = AccountId::from_str(&computekey_str).unwrap();

            processed_computekeys.push((computekey_account, (*amount, *uid)));
        }

        processed_stakes.push((personalkey_account, processed_computekeys));
    }

    let mut balances_issuance: u128 = 0;
    let mut processed_balances: Vec<(AccountId, u128)> = Vec::new();
    for (key_str, amount) in old_state.balances.iter() {
        let key_account = AccountId::from_str(&key_str).unwrap();
        let processed_amount: u128 = amount.parse::<u128>().unwrap();

        processed_balances.push((key_account, processed_amount));
        balances_issuance += processed_amount;
    }

    // Give front-ends necessary data to present to users
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), "BASED".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 32323.into());

    Ok(ChainSpec::from_genesis(
        // Name
        "Basedai",
        // ID
        "basedai",
        ChainType::Live,
        move || {
            prometheus_genesis(
                wasm_binary,
                // Initial PoA authorities (Validators)
                // aura | grandpa
                vec![
                    // Keys authority - aura, grandpa
                    //authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob"),
                    authority_keys_from_ss58(
                        "5FZXhRrBEtAtR4DRdMtb8X7o4KLSA5qiKLoQEJLV12s61zeN",
                        "5GVPYsCHtvkUkd3XWpjKw5ar4hYnTXLuWcmzEeFeZ6ouH15Q",
                    ), // key 1
                    authority_keys_from_ss58(
                        "5HKhaJJ2iDPJcT3uBA1XUr6EW8DyiJyifH6sn2Rvb4yx3ybx",
                        "5CkNG4bqThUjxgvsvFFEkqsMyoMexy2ribeHmfvBLGAGKSBj"
                    ),
                    authority_keys_from_ss58(
                        "5DtuJKrV7q3EBjipdf55rv9q2tNTggotCHWVvQhz2kaFpqhf",
                        "5FoGrFQwirWShheb5QHRiiS9fUdUyZ1wjnf2Cv8ttuCRymSv"
                    ),
                    authority_keys_from_ss58(
                        "5FyPRDFHK1UxEHn6dsahwbhkhgcK67um8kYV9p7NmfdiMzJD",
                        "5DnT9PPuvy3Uo3DA56M4R81Em83AFGGR6dBzMyuudMwsUZWg"
                    ),
                    authority_keys_from_ss58(
                        "5Dd7n9ErxX97k9F2eGxvjs25pxfxtHWxWZ3n4Ddo4xv7wWLr",
                        "5DrGLJFZ6QqhpFTWKq8YAsNGk1q2Fps4BbomAc5CnQycD1PA"
                    ),
                    authority_keys_from_ss58(
                        "5FH9EyLhAkB57r4QLZ3J9nYHg4hffSvjuNTkAPtCLepDzaiQ",
                        "5Cxerm8RSmNBozE2PFPa6jRCKo9hvEFBHK9XvkAMu2cV3ZNa"
                    ),
                    authority_keys_from_ss58(
                        "5GZ2Mf7bzN7JxnmQz1ndUysZoRHT28EyTrktGP9TScuHBCsY",
                        "5E8CzvKipm3fLXcpFbbHVeAqc4QYE3qe6ynUQj2X7XvfFf59"
                    ),
                ],
                // Sudo account
                AccountId::from_str("1d610e9a97119c3506a5e8744e83f6a6f86550a8").unwrap(),
                // Pre-funded accounts
                vec![],
                true,
                processed_stakes.clone(),
                processed_balances.clone(),
                balances_issuance,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        Some("basedai"),
        None,
        // Properties
        Some(properties),
        // Extensions
        None,
    ))
}

pub fn cyan_testnet_config() -> Result<ChainSpec, String> {
    let path: PathBuf = std::path::PathBuf::from("./snapshot.json");
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    // We mmap the file into memory first, as this is *a lot* faster than using
    // `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
    let file = File::open(&path)
        .map_err(|e| format!("Error opening genesis file `{}`: {}", path.display(), e))?;

    // SAFETY: `mmap` is fundamentally unsafe since technically the file can change
    //         underneath us while it is mapped; in practice it's unlikely to be a problem
    let bytes = unsafe {
        memmap2::Mmap::map(&file)
            .map_err(|e| format!("Error mmaping genesis file `{}`: {}", path.display(), e))?
    };

    let old_state: PersonalkeyComputekeys =
        json::from_slice(&bytes).map_err(|e| format!("Error parsing genesis file: {}", e))?;

    let mut processed_stakes: Vec<(AccountId, Vec<(AccountId, (u128, u16))>)> = Vec::new();
    for (personalkey_str, computekeys) in old_state.stakes.iter() {
        if let Ok(personalkey_account) = AccountId::from_str(&personalkey_str) {

			let mut processed_computekeys: Vec<(AccountId, (u128, u16))> = Vec::new();

			for (computekey_str, amount_uid) in computekeys.iter() {
				let (amount, uid) = amount_uid;
				let computekey_account = AccountId::from_str(&computekey_str).unwrap();

				processed_computekeys.push((computekey_account, (*amount, *uid)));
			}

			processed_stakes.push((personalkey_account, processed_computekeys));
		}
    }

    let mut balances_issuance: u128 = 0;
    let mut processed_balances: Vec<(AccountId, u128)> = Vec::new();
    for (key_str, amount) in old_state.balances.iter() {
        let key_account = AccountId::from_str(&key_str).unwrap();
        let processed_amount: u128 = amount.parse::<u128>().unwrap() * 1000;

        processed_balances.push((key_account, processed_amount));
        balances_issuance += processed_amount;
    }

    // Give front-ends necessary data to present to users
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), "BASED".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 32323.into());

    Ok(ChainSpec::from_genesis(
        // Name
        "Basedai",
        // ID
        "basedai",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities (Validators)
                // aura | grandpa
                vec![
                    // Keys for debug
                    //authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob"),
                    authority_keys_from_ss58(
                        "5HgeiRzZKZvyTfa4v8SA2LYfeKSG38AaHCyUE76FqH9Jtqa2",
                        "5CeSxXiW5WiLMtgbuqYD7uEB7C2znRiQi1e5GiD1h7GcFxBD",

                    ), // key 1
                    authority_keys_from_ss58(
                        "5HK8UEmv8yW3CbAzspKK7QKKM4smR3znfCVvKoD8bU1zbtcE",
                        "5HW8ESENXRVXd6sAVQovax5wswaGb3g7fx4RwGu7cnCDBLUn",
                    ), // key 2
                ],
                // Sudo account
                AccountId::from_str("a8cb782a9cb2c2f89b84b15b4bf04fb879884bf5").unwrap(),
                // Pre-funded accounts
                vec![],
                true,
                processed_stakes.clone(),
                vec![
                    (
                        AccountId::from_str("a8cb782a9cb2c2f89b84b15b4bf04fb879884bf5").unwrap(),
                        1000_000_000_000_000_000_000,
                    ),
                ]
                ,
                balances_issuance,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        Some("basedai"),
        None,
        // Properties
        Some(properties),
        // Extensions
        None,
    ))
}

pub fn localnet_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    // Give front-ends necessary data to present to users
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), "BASED".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), 13116.into());

    Ok(ChainSpec::from_genesis(
        // Name
        "Basedai",
        // ID
        "basedai",
        ChainType::Development,
        move || {
            localnet_genesis(
                wasm_binary,
                // Initial PoA authorities (Validators)
                // aura | grandpa
                vec![
                    // Keys for debug
                    authority_keys_from_seed("Alice"),
                    authority_keys_from_seed("Bob"),
                ],
                // Pre-funded accounts
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        Some("basedai"),
        None,
        // Properties
        Some(properties),
        // Extensions
        None,
    ))
}

fn localnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    _enable_println: bool,
) -> GenesisConfig {
    let mut balances = vec![
        (
            AccountId::from_str("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac").unwrap(),
            1000_000_000_000_000_000_000,
        ), // Alith
        (
            AccountId::from_str("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0").unwrap(),
            1_000_000_000_000,
        ), // Baltathar
        (
            AccountId::from_str("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc").unwrap(),
            1_000_000_000_000,
        ), // Charleth
        (
            AccountId::from_str("773539d4Ac0e786233D90A233654ccEE26a613D9").unwrap(),
            2_000_000_000_000_000_000, // 2 BASED
        ), // Dorothy
        (
            AccountId::from_str("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB").unwrap(),
            2_000_000_000_000_000_000, // 2 BASED
        ), // Ethan
        (
            AccountId::from_str("C0F0f4ab324C46e55D02D0033343B4Be8A55532d").unwrap(),
            2_000_000_000_000_000_000, // 2 BASED
        ), // Faith
    ];

    // Check if the environment variable is set
    if let Ok(bt_wallet) = env::var("BT_DEFAULT_TOKEN_WALLET") {
        if let Ok(decoded_wallet) = AccountId::from_str(&bt_wallet) {
            balances.push((decoded_wallet, 1_000_000_000_000_000));
        } else {
            eprintln!("Invalid format for BT_DEFAULT_TOKEN_WALLET.");
        }
    }

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
        },
        balances: BalancesConfig { balances },
        aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        },
        sudo: SudoConfig {
            key: Some(AccountId::from_str("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac").unwrap()), //Alith
        },
        transaction_payment: Default::default(),
        based_node: Default::default(),
        triumvirate: TriumvirateConfig {
            // Add initial authorities as collective members
            members: Default::default(), //initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
            phantom: Default::default(),
        },
        triumvirate_members: TriumvirateMembersConfig {
            members: Default::default(),
            phantom: Default::default(),
        },
        senate_members: SenateMembersConfig {
            members: Default::default(),
            phantom: Default::default(),
        },
        // EVM compatibility
        // evm_chain_id: EVMChainIdConfig { chain_id },
        evm_chain_id: basednode_runtime::EVMChainIdConfig {
            chain_id: SS58Prefix::get() as u64,
        },
        evm: EVMConfig {
            accounts: Default::default(),
        },
        ethereum: Default::default(),
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
    }
}

fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    _root_key: AccountId,
    _endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
    stakes: Vec<(AccountId, Vec<(AccountId, (u128, u16))>)>,
    balances: Vec<(AccountId, u128)>,
    balances_issuance: u128,
) -> GenesisConfig {
    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            //balances: balances.iter().cloned().map(|k| k).collect(),
            balances: balances.iter().cloned().map(|k| k).collect(),
        },
        aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        },
        sudo: SudoConfig {
            key: Some(AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap()),
        },
        transaction_payment: Default::default(),
        based_node: Default::default(),
        triumvirate: TriumvirateConfig {
            // Add initial authorities as collective members
            members: Default::default(), //initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
            phantom: Default::default(),
        },
        triumvirate_members: TriumvirateMembersConfig {
            members: Default::default(),
            phantom: Default::default(),
        },
        senate_members: SenateMembersConfig {
            members: Default::default(),
            phantom: Default::default(),
        },
        // EVM compatibility
        evm_chain_id: basednode_runtime::EVMChainIdConfig {
            chain_id: SS58Prefix::get() as u64,
        },
        evm: EVMConfig {
            accounts: Default::default(),
        },
        ethereum: Default::default(),
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
    }
}

// Configure initial storage state for FRAME modules.
fn prometheus_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    _root_key: AccountId,
    _endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
    stakes: Vec<(AccountId, Vec<(AccountId, (u128, u16))>)>,
    balances: Vec<(AccountId, u128)>,
    balances_issuance: u128,
) -> GenesisConfig {
    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
        },
        balances: BalancesConfig {
            balances: balances.iter().cloned().map(|k| k).collect()
        },
        aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        },
        sudo: SudoConfig {
            key: Some(AccountId::from_str("1d610e9a97119c3506a5e8744e83f6a6f86550a8").unwrap()),
        },
        transaction_payment: Default::default(),
        based_node: Default::default(),
        triumvirate: TriumvirateConfig {
            // Add initial authorities as collective members
            members: Default::default(), //initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
            phantom: Default::default(),
        },
        triumvirate_members: TriumvirateMembersConfig {
            members: Default::default(),
            phantom: Default::default(),
        },
        senate_members: SenateMembersConfig {
            members: Default::default(),
            phantom: Default::default(),
        },
        // EVM compatibility
        evm_chain_id: basednode_runtime::EVMChainIdConfig {
            chain_id: SS58Prefix::get() as u64,
        },
        evm: EVMConfig {
            accounts: Default::default(),
        },
        ethereum: Default::default(),
        dynamic_fee: Default::default(),
        base_fee: Default::default(),
    }
}
