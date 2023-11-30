use bonsaidb::core::schema::SerializedView;

use crate::{
	database::views::release::ReleaseByNameAndArtist,
	errors::Result,
	models::{state::AppState, tauri::release::ReleaseEntity},
};

#[tauri::command]
#[tracing::instrument(skip_all, err(Debug))]
pub async fn get_releases(library_name: String, app_state: tauri::State<'_, AppState>) -> Result<Vec<ReleaseEntity>> {
	let db_lock = app_state.db.lock().await;
	let database = db_lock.as_ref().unwrap();
	let database = &database.0;

	let entries = ReleaseByNameAndArtist::entries_async(database)
		.query_with_collection_docs()
		.await?;

	let docs = entries
		.documents
		.into_iter()
		.map(|(id, e)| ReleaseEntity::new(id, e.contents))
		.collect::<Vec<_>>();

	Ok(docs)
}
