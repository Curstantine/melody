use crate::database::models::label::Label;

#[derive(Debug)]
pub struct TempLabel {
	pub name: String,
}

impl TempLabel {
	pub fn into_label(self, library_ids: Vec<u32>) -> Label {
		Label {
			name: self.name,
			library_ids,
		}
	}
}
