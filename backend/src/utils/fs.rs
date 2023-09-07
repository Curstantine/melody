use futures::{stream, Stream, StreamExt};
use std::path::PathBuf;
use tokio::fs::{self, DirEntry};

use crate::errors::Result;

pub fn walkdir(path: impl Into<PathBuf>) -> impl Stream<Item = Result<DirEntry>> + Send + 'static {
	async fn one_level(path: PathBuf, to_visit: &mut Vec<PathBuf>) -> Result<Vec<DirEntry>> {
		let mut dir = fs::read_dir(path).await?;
		let mut files = Vec::new();

		while let Some(child) = dir.next_entry().await? {
			if child.metadata().await?.is_dir() {
				to_visit.push(child.path());
			} else {
				files.push(child)
			}
		}

		Ok(files)
	}

	stream::unfold(vec![path.into()], |mut to_visit| async {
		let path = to_visit.pop()?;
		let file_stream = match one_level(path, &mut to_visit).await {
			Ok(files) => stream::iter(files).map(Ok).left_stream(),
			Err(e) => stream::once(async { Err(e) }).right_stream(),
		};

		Some((file_stream, to_visit))
	})
	.flatten()
}

#[cfg(test)]
mod tests {
	use futures::StreamExt;
	use std::path::Path;

	#[tokio::test]
	async fn test_walk_dir() {
		let target = Path::new("./target");
		let result = super::walkdir(target);

		result
			.for_each(|entry| async {
				let entry = entry.unwrap();
				let path = entry.path();
				let metadata = entry.metadata().await.unwrap();

				if metadata.is_dir() {
					println!("dir: {}", path.display());
				} else {
					println!("file: {}", path.display());
				}
			})
			.await;
	}
}
