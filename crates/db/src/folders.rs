use std::time::{SystemTime, UNIX_EPOCH};

use turso::params;

use crate::{DbError, LibraryDatabase};

impl LibraryDatabase {
    pub async fn get_library_folders(&self) -> Result<Vec<String>, DbError> {
        let mut rows = self
            .conn
            .query("SELECT path FROM library_folders ORDER BY id ASC", ())
            .await?;

        let mut folders = Vec::new();
        while let Some(row) = rows.next().await? {
            let path: String = row.get(0)?;
            folders.push(path);
        }
        Ok(folders)
    }

    /// Adds a directory path to the library folders list if not already present
    pub async fn add_library_folder(&self, path: &str) -> Result<bool, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO library_folders (path, added_timestamp) VALUES (?1, ?2)",
                params![path.to_string(), now],
            )
            .await?;
        Ok(affected > 0)
    }

    /// Removes a directory path from the library folders list
    pub async fn remove_library_folder(&self, path: &str) -> Result<bool, DbError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM library_folders WHERE path = ?1",
                params![path.to_string()],
            )
            .await?;
        Ok(affected > 0)
    }
}
