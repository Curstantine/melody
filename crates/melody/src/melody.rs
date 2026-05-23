use gpui::{Context, IntoElement, Render, Window, div, prelude::*};

pub struct MainApp {}

impl Render for MainApp {
	fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
		div().flex()
	}
}
