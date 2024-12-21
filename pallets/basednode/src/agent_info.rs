use super::*;
use frame_support::storage::IterableStorageDoubleMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct AgentInfo<T: Config> {
    computekey: T::AccountId,
    personalkey: T::AccountId,
    uid: Compact<u16>,
    netuid: Compact<u16>,
    active: bool,
    brainport_info: BrainportInfo,
    prometheus_info: PrometheusInfo,
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of personalkey to stake on this agent/computekey (includes delegations)
    rank: Compact<u16>,
    emission: Compact<u64>,
    incentive: Compact<u16>,
    consensus: Compact<u16>,
    trust: Compact<u16>,
    validator_trust: Compact<u16>,
    dividends: Compact<u16>,
    last_update: Compact<u64>,
    validator_permit: bool,
    weights: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, weight)
    bonds: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, bond)
    pruning_score: Compact<u16>,
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct AgentInfoLite<T: Config> {
    computekey: T::AccountId,
    personalkey: T::AccountId,
    uid: Compact<u16>,
    netuid: Compact<u16>,
    active: bool,
    brainport_info: BrainportInfo,
    prometheus_info: PrometheusInfo,
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of personalkey to stake on this agent/computekey (includes delegations)
    rank: Compact<u16>,
    emission: Compact<u64>,
    incentive: Compact<u16>,
    consensus: Compact<u16>,
    trust: Compact<u16>,
    validator_trust: Compact<u16>,
    dividends: Compact<u16>,
    last_update: Compact<u64>,
    validator_permit: bool,
    // has no weights or bonds
    pruning_score: Compact<u16>,
}

impl<T: Config> Pallet<T> {
	pub fn get_agents(netuid: u16) -> Vec<AgentInfo<T>> {
        if !Self::if_brain_exist(netuid) {
            return Vec::new();
        }

        let mut agents = Vec::new();
        let n = Self::get_brain_n(netuid);
        for uid in 0..n {
            let uid = uid;
            let netuid = netuid;

            let _agent = Self::get_agent_brain_exists(netuid, uid);
            let agent;
            if _agent.is_none() {
                break; // No more agents
            } else {
                // No error, computekey was registered
                agent = _agent.expect("agent should exist");
            }

            agents.push( agent );
        }
        agents
	}

    fn get_agent_brain_exists(netuid: u16, uid: u16) -> Option<AgentInfo<T>> {
        let _computekey = Self::get_computekey_for_net_and_uid(netuid, uid);
        let computekey;
        if _computekey.is_err() {
            return None;
        } else {
            // No error, computekey was registered
            computekey = _computekey.expect("Computekey should exist");
        }

        let brainport_info = Self::get_brainport_info( netuid, &computekey.clone() );

        let prometheus_info = Self::get_prometheus_info( netuid, &computekey.clone() );


        let personalkey = Owner::<T>::get( computekey.clone() ).clone();

        let active = Self::get_active_for_uid( netuid, uid as u16 );
        let rank = Self::get_rank_for_uid( netuid, uid as u16 );
        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let consensus = Self::get_consensus_for_uid( netuid, uid as u16 );
        let trust = Self::get_trust_for_uid( netuid, uid as u16 );
        let validator_trust = Self::get_validator_trust_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let pruning_score = Self::get_pruning_score_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );
        let validator_permit = Self::get_validator_permit_for_uid( netuid, uid as u16 );

        let weights = <Weights<T>>::get(netuid, uid).iter()
            .filter_map(|(i, w)| if *w > 0 { Some((i.into(), w.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();

        let bonds = <Bonds<T>>::get(netuid, uid).iter()
            .filter_map(|(i, b)| if *b > 0 { Some((i.into(), b.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();

        let stake: Vec<(T::AccountId, Compact<u64>)> = < Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64> >::iter_prefix( computekey.clone() )
            .map(|(personalkey, stake)| (personalkey, stake.into()))
            .collect();

        let agent = AgentInfo {
            computekey: computekey.clone(),
            personalkey: personalkey.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            active,
            brainport_info,
            prometheus_info,
            stake,
            rank: rank.into(),
            emission: emission.into(),
            incentive: incentive.into(),
            consensus: consensus.into(),
            trust: trust.into(),
            validator_trust: validator_trust.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            validator_permit,
            weights,
            bonds,
            pruning_score: pruning_score.into()
        };

        return Some(agent);
    }

    pub fn get_agent(netuid: u16, uid: u16) -> Option<AgentInfo<T>> {
        if !Self::if_brain_exist(netuid) {
            return None;
        }

        let agent = Self::get_agent_brain_exists(netuid, uid);
        agent
	}

    fn get_agent_lite_brain_exists(netuid: u16, uid: u16) -> Option<AgentInfoLite<T>> {
        let _computekey = Self::get_computekey_for_net_and_uid(netuid, uid);
        let computekey;
        if _computekey.is_err() {
            return None;
        } else {
            // No error, computekey was registered
            computekey = _computekey.expect("Computekey should exist");
        }

        let brainport_info = Self::get_brainport_info( netuid, &computekey.clone() );

        let prometheus_info = Self::get_prometheus_info( netuid, &computekey.clone() );


        let personalkey = Owner::<T>::get( computekey.clone() ).clone();

        let active = Self::get_active_for_uid( netuid, uid as u16 );
        let rank = Self::get_rank_for_uid( netuid, uid as u16 );
        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let consensus = Self::get_consensus_for_uid( netuid, uid as u16 );
        let trust = Self::get_trust_for_uid( netuid, uid as u16 );
        let validator_trust = Self::get_validator_trust_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let pruning_score = Self::get_pruning_score_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );
        let validator_permit = Self::get_validator_permit_for_uid( netuid, uid as u16 );

        let stake: Vec<(T::AccountId, Compact<u64>)> = < Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64> >::iter_prefix( computekey.clone() )
            .map(|(personalkey, stake)| (personalkey, stake.into()))
            .collect();

        let agent = AgentInfoLite {
            computekey: computekey.clone(),
            personalkey: personalkey.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            active,
            brainport_info,
            prometheus_info,
            stake,
            rank: rank.into(),
            emission: emission.into(),
            incentive: incentive.into(),
            consensus: consensus.into(),
            trust: trust.into(),
            validator_trust: validator_trust.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            validator_permit,
            pruning_score: pruning_score.into()
        };

        return Some(agent);
    }

    pub fn get_agents_lite(netuid: u16) -> Vec<AgentInfoLite<T>> {
         if !Self::if_brain_exist(netuid) {
            return Vec::new();
        }

        let mut agents: Vec<AgentInfoLite<T>> = Vec::new();
        let n = Self::get_brain_n(netuid);
        for uid in 0..n {
            let uid = uid;

            let _agent = Self::get_agent_lite_brain_exists(netuid, uid);
            let agent;
            if _agent.is_none() {
                break; // No more agents
            } else {
                // No error, computekey was registered
                agent = _agent.expect("Agent should exist");
            }

            agents.push( agent );
        }
        agents
    }

    pub fn get_agent_lite(netuid: u16, uid: u16) -> Option<AgentInfoLite<T>> {
        if !Self::if_brain_exist(netuid) {
            return None;
        }

        let agent = Self::get_agent_lite_brain_exists(netuid, uid);
        agent
   }
}

