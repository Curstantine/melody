use bonsaidb::{core::keyvalue::AsyncKeyValue, local::AsyncDatabase};

use crate::{database::constants::KEY_SCAN_LOCATIONS, errors::Result};

pub async fn set_scan_locations(database: &AsyncDatabase, scan_locations: &Vec<String>) -> Result<()> {
	database.set_key(KEY_SCAN_LOCATIONS, scan_locations).await?;

	Ok(())
}

pub async fn get_scan_locations(database: &AsyncDatabase) -> Result<Option<Vec<String>>> {
	let x = database
		.get_key(KEY_SCAN_LOCATIONS)
		.await?
		.map(|rx| rx.deserialize::<Vec<String>>())
		.transpose()?;

	Ok(x)
}
