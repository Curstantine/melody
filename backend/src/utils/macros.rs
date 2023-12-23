#[macro_export]
macro_rules! parse_str {
	($str:expr, $type:ty) => {
		$str.parse::<$type>()
			.map_err(|e| Error::from(e).set_str_data($str.to_string()))
	};
}
