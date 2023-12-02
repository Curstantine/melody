use bonsaidb::core::schema::{SerializedCollection, SerializedView};

use crate::{
	database::{models::release::Release, views::release::ReleaseByNameAndArtist},
	errors::Result,
	models::{state::AppState, tauri::release::ReleaseEntity},
};

#[tauri::command]
#[tracing::instrument(skip(app_state), err(Debug))]
pub async fn get_releases(library_id: u64, app_state: tauri::State<'_, AppState>) -> Result<Vec<ReleaseEntity>> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let entries = ReleaseByNameAndArtist::entries_async(database)
		.query_with_docs()
		.await?;
	let mut releases = Vec::with_capacity(entries.len());

	for mapping in &entries {
		let id = mapping.document.header.id.deserialize::<u64>()?;
		let release = Release::document_contents(mapping.document)?;
		releases.push(ReleaseEntity::new(id, release))
	}

	Ok(releases)
}
