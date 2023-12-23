use std::{
	io,
	path::{Path, PathBuf},
};

use crate::errors::Result;

pub fn walkdir_sync<M>(path: &Path, match_fn: M) -> Result<Vec<PathBuf>>
where
	M: Fn(&Path) -> bool + Copy,
{
	fn one_level(path: &Path, matcher: impl Fn(&Path) -> bool) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
		let mut dir = std::fs::read_dir(path)?;
		let mut files = Vec::new();
		let mut to_visit = Vec::new();

		while let Some(child) = dir.next().transpose()? {
			if child.metadata()?.is_dir() {
				to_visit.push(child.path());
			} else if matcher(&child.path()) {
				files.push(child.path());
			}
		}

		Ok((files, to_visit))
	}

	let files = one_level(path, match_fn).and_then(|(mut files, mut to_visit)| {
		while let Some(path) = to_visit.pop() {
			let (mut new_files, mut new_to_visit) = one_level(path.as_path(), match_fn)?;

			files.append(&mut new_files);
			to_visit.append(&mut new_to_visit);
		}

		Ok(files)
	})?;

	Ok(files)
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use crate::errors::Result;

	#[test]
	fn test_walk_dir_sync() -> Result<()> {
		let target = Path::new("./target");
		let paths = super::walkdir_sync(target, |_| true).unwrap();

		for path in paths {
			assert!(path.exists(), "Path {} does not exist", path.display());
			assert!(path.is_file(), "Path {} is not a file", path.display());
		}

		Ok(())
	}
}
