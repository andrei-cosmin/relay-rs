use std::{sync::Arc, time::Duration};

use axum::{Extension, Router};
use relay_core::{Config, Controller, Resources, Shutdown};

use crate::{Settings, SettingsApi};

#[derive(Clone)]
pub struct SettingsController {
    current: Arc<Settings>,
}

impl Controller for SettingsController {
    fn build(resources: &Resources) -> Self {
        Self { current: resources.get::<Settings>() }
    }

    fn expose(&self, router: Router) -> Router {
        let api: Arc<dyn SettingsApi> = Arc::new(self.clone());
        router.layer(Extension(api))
    }
}

impl SettingsApi for SettingsController {
    fn settings(&self) -> Settings {
        Settings::clone(&self.current)
    }

    fn save(&self, settings: Settings) -> Result<(), String> {
        settings.save().map_err(|error| error.message)
    }

    fn restart(&self) {
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            Shutdown::request();
        });
    }
}
