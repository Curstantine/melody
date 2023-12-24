#[macro_export]
macro_rules! parse_str {
	($str:expr, $type:ty) => {
		$str.parse::<$type>()
	};
}
