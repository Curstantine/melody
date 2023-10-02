pub mod fs;
pub mod symphonia;

#[inline]
pub fn get_opt_vec_len<T>(x: &Option<Vec<T>>) -> usize {
	match x {
		Some(y) => y.len(),
		None => 0,
	}
}
