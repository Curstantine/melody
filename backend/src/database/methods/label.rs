use bonsaidb::{
	core::schema::{SerializedCollection, SerializedView},
	local::AsyncDatabase,
};

use crate::{
	database::{models::label::Label, views::label::LabelByName},
	errors::{Result, pre::database_entry_library_id_exists},
};

pub async fn get_by_name(database: &AsyncDatabase, name: &str) -> Result<Option<u64>> {
	let matches = LabelByName::entries_async(database).with_key(name).query().await?;
	Ok(matches.first().map(|e| e.source.id))
}

pub async fn add_library_id(database: &AsyncDatabase, label_id: u64, library_id: u32) -> Result<()> {
	match Label::get_async(&label_id, database).await? {
		Some(mut label) if label.contents.library_ids.contains(x) {
			return Err(database_entry_library_id_exists("labels", label_id, library_id))
		}
		_ => (),
	}
}

pub async fn get_or_insert(database: &AsyncDatabase, label: Label) -> Result<u64> {
	let matches = LabelByName::entries_async(database)
		.with_key(&label.name)
		.query()
		.await?;

	let id = if let Some(label) = matches.first() {
		label.source.id
	} else {
		let label = label.push_into_async(database).await?;
		label.header.id
	};

	Ok(id)
}

#[cfg(test)]
mod test {
	use bonsaidb::core::schema::{SerializedCollection, SerializedView};

	use crate::{
		database::{methods::label::get_or_insert, models::label::Label, views::label::LabelByName, Database},
		errors::Result,
	};

	#[tokio::test]
	async fn test_by_name() -> Result<()> {
		let database = Database::testing().await?;
		let db = database.0;

		for i in 0..10 {
			let label = Label {
				library_ids: vec![],
				name: format!("Label {i}"),
			};

			label.push_into_async(&db).await?;
		}

		let len = LabelByName::entries_async(&db).reduce().await?;
		assert_eq!(len, 10);

		let x = LabelByName::entries_async(&db).with_key("Label 1").query().await?;
		assert_eq!(x.len(), 1);

		Ok(())
	}

	#[tokio::test]
	async fn test_get_or_insert() -> Result<()> {
		let db = Database::testing().await?;
		let dbx = db.0;

		let label = Label {
			library_ids: vec![],
			name: "Label 1".to_string(),
		};
		let doc = label.push_into_async(&dbx).await?;

		let label = Label {
			library_ids: vec![],
			name: "Label 1".to_string(),
		};
		let result = get_or_insert(&dbx, label).await?;
		assert_eq!(result, doc.header.id);

		Ok(())
	}
}
