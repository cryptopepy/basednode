use super::*;
use frame_support::inherent::Vec;
use frame_support::sp_std::vec;


impl<T: Config> Pallet<T> {

    // ---- The implementation for the extrinsic serve_brainport which sets the ip endpoint information for a uid on a network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'version' (u64):
    // 		- The basedai version identifier.
    //
    // 	* 'ip' (u64):
    // 		- The endpoint ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The endpoint port information as a u16 encoded integer.
    //
    // 	* 'ip_type' (u8):
    // 		- The endpoint ip version as a u8, 4 or 6.
    //
    // 	* 'protocol' (u8):
    // 		- UDP:1 or TCP:0
    //
    // 	* 'placeholder1' (u8):
    // 		- Placeholder for further extra params.
    //
    // 	* 'placeholder2' (u8):
    // 		- Placeholder for further extra params.
    //
    // # Event:
    // 	* BrainportServed;
    // 		- On successfully serving the brainport info.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpType':
    // 		- The ip type is not 4 or 6.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //
    pub fn do_serve_brainport(
        origin: T::RuntimeOrigin,
		netuid: u16,
        version: u32,
        ip: u128,
        port: u16,
        ip_type: u8,
        protocol: u8,
		placeholder1: u8,
		placeholder2: u8,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (computekey) signature.
        let computekey_id = ensure_signed(origin)?;

        // --- 2. Ensure the computekey is registered somewhere.
        ensure!( Self::is_computekey_registered_on_any_network( &computekey_id ), Error::<T>::NotRegistered );

        // --- 3. Check the ip signature validity.
        ensure!( Self::is_valid_ip_type(ip_type), Error::<T>::InvalidIpType );
        ensure!( Self::is_valid_ip_address(ip_type, ip), Error::<T>::InvalidIpAddress );

        // --- 4. Get the previous brainport information.
        let mut prev_brainport = Self::get_brainport_info( netuid, &computekey_id );
        let current_block:u64 = Self::get_current_block_as_u64();
        ensure!( Self::brainport_passes_rate_limit( netuid, &prev_brainport, current_block ), Error::<T>::ServingRateLimitExceeded );

        // --- 6. We insert the brainport meta.
        prev_brainport.block = Self::get_current_block_as_u64();
        prev_brainport.version = version;
        prev_brainport.ip = ip;
        prev_brainport.port = port;
        prev_brainport.ip_type = ip_type;
        prev_brainport.protocol = protocol;
        prev_brainport.placeholder1 = placeholder1;
        prev_brainport.placeholder2 = placeholder2;

		// --- 7. Validate brainport data with delegate func
		let brainport_validated = Self::validate_brainport_data(&prev_brainport);
		ensure!( brainport_validated.is_ok(), brainport_validated.err().unwrap_or(Error::<T>::InvalidPort) );

        Brainports::<T>::insert( netuid, computekey_id.clone(), prev_brainport );

        // --- 8. We deposit brainport served event.
        log::info!("BrainportServed( computekey:{:?} ) ", computekey_id.clone() );
        Self::deposit_event(Event::BrainportServed( netuid, computekey_id ));

        // --- 9. Return is successful dispatch.
        Ok(())
    }

    // ---- The implementation for the extrinsic serve_prometheus.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'version' (u64):
    // 		- The basedai version identifier.
    //
    // 	* 'ip' (u64):
    // 		- The prometheus ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The prometheus port information as a u16 encoded integer.
    //
    // 	* 'ip_type' (u8):
    // 		- The prometheus ip version as a u8, 4 or 6.
    //
    // # Event:
    // 	* PrometheusServed;
    // 		- On successfully serving the brainport info.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpType':
    // 		- The ip type is not 4 or 6.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //
    pub fn do_serve_prometheus(
        origin: T::RuntimeOrigin,
		netuid: u16,
        version: u32,
        ip: u128,
        port: u16,
        ip_type: u8,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (computekey) signature.
        let computekey_id = ensure_signed(origin)?;

        // --- 2. Ensure the computekey is registered somewhere.
        ensure!( Self::is_computekey_registered_on_any_network( &computekey_id ), Error::<T>::NotRegistered );

        // --- 3. Check the ip signature validity.
        ensure!( Self::is_valid_ip_type(ip_type), Error::<T>::InvalidIpType );
        ensure!( Self::is_valid_ip_address(ip_type, ip), Error::<T>::InvalidIpAddress );

        // --- 5. We get the previous brainport info assoicated with this ( netuid, uid )
        let mut prev_prometheus = Self::get_prometheus_info( netuid, &computekey_id );
        let current_block:u64 = Self::get_current_block_as_u64();
        ensure!( Self::prometheus_passes_rate_limit( netuid, &prev_prometheus, current_block ), Error::<T>::ServingRateLimitExceeded );

        // --- 6. We insert the prometheus meta.
        prev_prometheus.block = Self::get_current_block_as_u64();
        prev_prometheus.version = version;
        prev_prometheus.ip = ip;
        prev_prometheus.port = port;
        prev_prometheus.ip_type = ip_type;

		// --- 7. Validate prometheus data with delegate func
		let prom_validated = Self::validate_prometheus_data(&prev_prometheus);
		ensure!( prom_validated.is_ok(), prom_validated.err().unwrap_or(Error::<T>::InvalidPort) );

		// --- 8. Insert new prometheus data
        Prometheus::<T>::insert( netuid, computekey_id.clone(), prev_prometheus );

        // --- 9. We deposit prometheus served event.
        log::info!("PrometheusServed( computekey:{:?} ) ", computekey_id.clone() );
        Self::deposit_event(Event::PrometheusServed( netuid, computekey_id ));

        // --- 10. Return is successful dispatch.
        Ok(())
    }

    /********************************
     --==[[  Helper functions   ]]==--
    *********************************/

    pub fn brainport_passes_rate_limit( netuid: u16, prev_brainport_info: &BrainportInfoOf, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_serving_rate_limit(netuid);
        let last_serve = prev_brainport_info.block;
        return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
    }

    pub fn prometheus_passes_rate_limit( netuid: u16, prev_prometheus_info: &PrometheusInfoOf, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_serving_rate_limit(netuid);
        let last_serve = prev_prometheus_info.block;
        return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
    }

    pub fn has_brainport_info( netuid: u16, computekey: &T::AccountId ) -> bool {
        return Brainports::<T>::contains_key( netuid, computekey );
    }

    pub fn has_prometheus_info( netuid: u16, computekey: &T::AccountId ) -> bool {
        return Prometheus::<T>::contains_key( netuid, computekey );
    }

    pub fn get_brainport_info( netuid: u16, computekey: &T::AccountId ) -> BrainportInfoOf {
        if Self::has_brainport_info( netuid, computekey ) {
            return Brainports::<T>::get( netuid, computekey ).unwrap();
        } else{
            return BrainportInfo {
                block: 0,
                version: 0,
                ip: 0,
                port: 0,
                ip_type: 0,
                protocol: 0,
                placeholder1: 0,
                placeholder2: 0
            }

        }
    }

    pub fn get_prometheus_info( netuid: u16, computekey: &T::AccountId ) -> PrometheusInfoOf {
        if Self::has_prometheus_info( netuid, computekey ) {
            return Prometheus::<T>::get( netuid, computekey ).unwrap();
        } else {
            return PrometheusInfo {
                block: 0,
                version: 0,
                ip: 0,
                port: 0,
                ip_type: 0,
            }

        }
    }

    pub fn is_valid_ip_type(ip_type: u8) -> bool {
        let allowed_values: Vec<u8> = vec![4, 6];
        return allowed_values.contains(&ip_type);
    }

    // @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges
    pub fn is_valid_ip_address(ip_type: u8, addr: u128) -> bool {
        if !Self::is_valid_ip_type(ip_type) {
            return false;
        }
        if addr == 0 {
            return false;
        }
        if ip_type == 4 {
            if addr == 0 { return false; }
            if addr >= u32::MAX as u128 { return false; }
            if addr == 0x7f000001 { return false; } // Localhost
        }
        if ip_type == 6 {
            if addr == 0x0 { return false; }
            if addr == u128::MAX { return false; }
            if addr == 1 { return false; } // IPv6 localhost
        }
        return true;
    }

	pub fn validate_brainport_data(brainport_info: &BrainportInfoOf) -> Result<bool, pallet::Error<T>> {
		if brainport_info.port.clamp(0, u16::MAX) <= 0 {
			return Err(Error::<T>::InvalidPort);
		}

		Ok(true)
	}

	pub fn validate_prometheus_data(prom_info: &PrometheusInfoOf) -> Result<bool, pallet::Error<T>> {
		if prom_info.port.clamp(0, u16::MAX) <= 0 {
			return Err(Error::<T>::InvalidPort);
		}

		Ok(true)
	}

}
