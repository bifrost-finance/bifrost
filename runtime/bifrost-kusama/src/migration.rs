use super::*;
use crate::sp_api_hidden_includes_construct_runtime::hidden_include::dispatch::GetStorageVersion;
#[allow(unused_imports)]
use frame_support::ensure;
use frame_support::traits::OnRuntimeUpgrade;
use node_primitives::traits::XcmDestWeightAndFeeHandler;

const LOG_TARGET: &str = "XCM-INTERFACE::migration";

pub struct XcmInterfaceMigration;
impl OnRuntimeUpgrade for XcmInterfaceMigration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// Check the storage version
		let onchain_version = XcmInterface::on_chain_storage_version();
		if onchain_version < 2 {
			// Transform storage values
			// We transform the storage values from the old into the new format.
			log::info!(
				target: LOG_TARGET,
				"Start to migrate XcmInterface storage XcmDestWeightAndFee..."
			);

			let count1 = bifrost_slp::XcmDestWeightAndFee::<Runtime>::iter().count();

			// 先将Xcm_interface的XcmDestWeightAndFee的值从旧的存储中取出来，
			// 然后设置到新的XcmWeightAndFee存储中,新增一个currency_id作为主key
			xcm_interface::XcmDestWeightAndFee::<Runtime>::iter().for_each(|(key, value)| {
				log::info!(
					target: LOG_TARGET,
					"Migrated to doublemap for {:?}, {:?}...",
					key,
					value
				);
				let _ = XcmInterface::set_xcm_dest_weight_and_fee(
					RelayCurrencyId::get(),
					key,
					Some(value),
				);
			});

			// get value from the old SLP XcmDestWeightAndFee storage and set it to the XcmInterface
			// storage
			let count2 = bifrost_slp::XcmDestWeightAndFee::<Runtime>::iter().count();

			// iterrate the old SLP XcmDestWeightAndFee storage
			bifrost_slp::XcmDestWeightAndFee::<Runtime>::iter().for_each(|(key1, key2, value)| {
				log::info!(
					target: LOG_TARGET,
					"Migrated to XcmInterface XcmWeightAndFee for {:?}, {:?}, {:?}...",
					key1,
					key2,
					value
				);
				// set the value to the new XcmInterface storage
				let _ = XcmInterface::set_xcm_dest_weight_and_fee(key1, key2, Some(value));
			// delete the old SLP XcmDestWeightAndFee storage
			bifrost_slp::XcmDestWeightAndFee::<Runtime>::remove(key1, key2);
		});

					// delete the old Xcm_interface XcmDestWeightAndFee storage
					xcm_interface::XcmDestWeightAndFee::<Runtime>::iter().for_each(|(key, _value)| {
						xcm_interface::XcmDestWeightAndFee::<Runtime>::remove(key);
					});

			// Update the storage version
			StorageVersion::new(2).put::<xcm_interface::Pallet<Runtime>>();

			let count = count1 + count2;
			// Return the consumed weight
			Weight::from(
				<Runtime as frame_system::Config>::DbWeight::get()
					.reads_writes(count as u64 + 1, count as u64 + 1),
			)
		} else {
			// We don't do anything here.
			Weight::zero()
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let xcm_interface_xcm_dest_weight_and_fee_cnt =
		xcm_interface::XcmDestWeightAndFee::<Runtime>::iter().count();
		// print out the pre-migrate storage count
		log::info!(
			target: LOG_TARGET,
			"XcmInterface XcmDestWeightAndFee pre-migrate storage count: {:?}",
			xcm_interface_xcm_dest_weight_and_fee_cnt
		);

		let slp_xcm_dest_weight_and_fee_cnt = bifrost_slp::XcmDestWeightAndFee::<Runtime>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"Slp XcmDestWeightAndFee pre-migrate storage count: {:?}",
			slp_xcm_dest_weight_and_fee_cnt
		);

		let cnt =
			(xcm_interface_xcm_dest_weight_and_fee_cnt + slp_xcm_dest_weight_and_fee_cnt) as u32;

		Ok(cnt.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(cnt: Vec<u8>) -> Result<(), &'static str> {
		let old_xcm_interface_xcm_weight_and_fee_cnt: u32 = Decode::decode(&mut cnt.as_slice())
			.expect("the state parameter should be something that was generated by pre_upgrade");

		let new_xcm_interface_xcm_weight_and_fee_cnt =
		xcm_interface::XcmWeightAndFee::<Runtime>::iter().count();
		// print out the post-migrate storage count
		log::info!(
			target: LOG_TARGET,
			"XcmInterface XcmWeightAndFee post-migrate storage count: {:?}",
			new_xcm_interface_xcm_weight_and_fee_cnt
		);
		ensure!(
			new_xcm_interface_xcm_weight_and_fee_cnt as u32 ==
				old_xcm_interface_xcm_weight_and_fee_cnt,
			"XcmInterface XcmWeightAndFee post-migrate storage count not match"
		);

		Ok(())
	}
}
