use std::sync::Arc;

use anyhow::Result;

use crate::domain::contracts::NodeStore;
use crate::domain::models::{AvecState, CalibrationResult};

pub struct CalibrationService {
    store: Arc<dyn NodeStore>,
}

impl CalibrationService {
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self { store }
    }

    pub async fn calibrate_async(
        &self,
        session_id: &str,
        stability: f32,
        friction: f32,
        logic: f32,
        autonomy: f32,
        trigger: &str,
    ) -> Result<CalibrationResult> {
        let current = AvecState {
            stability,
            friction,
            logic,
            autonomy,
        };

        let previous = self.store.get_last_avec_async(session_id).await?;
        let history = self.store.get_trigger_history_async(session_id).await?;
        let is_first = previous.is_none();
        let baseline = previous.unwrap_or(current);

        self.store
            .store_calibration_async(session_id, current, trigger)
            .await?;

        let mut trigger_history = history;
        trigger_history.push(trigger.to_string());

        Ok(CalibrationResult {
            previous_avec: baseline,
            delta: current.drift_from(baseline),
            drift_classification: current.classify_drift(baseline),
            trigger: trigger.to_string(),
            trigger_history,
            is_first_calibration: is_first,
        })
    }
}
