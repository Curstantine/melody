use std::{
	io::Result as IoResult,
	path::{Path, PathBuf},
};

pub fn walkdir_sync<P, M>(path: P, match_fn: M) -> IoResult<Vec<PathBuf>>
where
	P: Into<PathBuf>,
	M: Fn(&Path) -> bool + Copy,
{
	fn one_level(path: PathBuf, matcher: impl Fn(&Path) -> bool) -> IoResult<(Vec<PathBuf>, Vec<PathBuf>)> {
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

	one_level(path.into(), match_fn).and_then(|(mut files, mut to_visit)| {
		while let Some(path) = to_visit.pop() {
			let (mut new_files, mut new_to_visit) = one_level(path, match_fn)?;

			files.append(&mut new_files);
			to_visit.append(&mut new_to_visit);
		}

		Ok(files)
	})
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use crate::errors::Result;

	#[test]
	fn test_walk_dir_sync() -> Result<()> {
		let target = Path::new("./target");
		let paths = super::walkdir_sync(target, |_| true)?;

		for path in paths {
			assert!(path.exists(), "Path {} does not exist", path.display());
			assert!(path.is_file(), "Path {} is not a file", path.display());
		}

		Ok(())
	}
}
