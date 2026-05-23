use std::{sync::OnceLock, time::Instant};

use assets::Assets;
use db::AppDatabase;
use gpui::{App, Application, Size, WindowOptions, prelude::*, px};

use crate::melody::MainApp;

mod melody;

static STARTUP_TIME: OnceLock<Instant> = OnceLock::new();
static APP_ID: &str = "moe.curstantine.melody";

fn main() {
	STARTUP_TIME.get_or_init(Instant::now);
	env_logger::init();

	let app = Application::new().with_assets(Assets);
	let app_db = match AppDatabase::new() {
		Ok(database) => database,
		Err(e) => {
			log::error!("Failed to initialize the database: {e}");
			return;
		}
	};

	app.run(|cx: &mut App| {
		cx.set_global(app_db);

		cx.open_window(
			WindowOptions {
				app_id: Some(APP_ID.into()),
				window_bounds: None,
				tabbing_identifier: None,
				window_min_size: Some(Size {
					width: px(360.0),
					height: px(240.0),
				}),
				..Default::default()
			},
			|_, cx| cx.new(|_| MainApp {}),
		)
		.unwrap();
	});
}
