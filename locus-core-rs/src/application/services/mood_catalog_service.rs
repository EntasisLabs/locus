use crate::domain::models::{AvecState, MoodCatalogResult, MoodPreset, MoodSwapPreview};

#[derive(Debug, Default, Clone, Copy)]
pub struct MoodCatalogService;

impl MoodCatalogService {
    pub fn new() -> Self {
        Self
    }

    pub fn get(
        &self,
        target_mood: Option<&str>,
        blend: f32,
        current_stability: Option<f32>,
        current_friction: Option<f32>,
        current_logic: Option<f32>,
        current_autonomy: Option<f32>,
    ) -> MoodCatalogResult {
        let presets = build_presets();
        let guide = "Choose mood -> hard swap direct values or soft swap with blend -> use resulting AVEC as active state -> recalibrate after heavy reasoning shifts.".to_string();

        let mut result = MoodCatalogResult {
            presets: presets.clone(),
            apply_guide: guide,
            swap_preview: None,
        };

        let Some(target) = target_mood else {
            return result;
        };

        let Some(mood) = presets
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(target))
            .cloned()
        else {
            return result;
        };

        let (
            Some(current_stability),
            Some(current_friction),
            Some(current_logic),
            Some(current_autonomy),
        ) = (
            current_stability,
            current_friction,
            current_logic,
            current_autonomy,
        )
        else {
            return result;
        };

        let normalized_blend = blend.clamp(0.0, 1.0);
        let current = AvecState {
            stability: current_stability,
            friction: current_friction,
            logic: current_logic,
            autonomy: current_autonomy,
        };

        let blended = AvecState {
            stability: current.stability * (1.0 - normalized_blend)
                + mood.avec.stability * normalized_blend,
            friction: current.friction * (1.0 - normalized_blend)
                + mood.avec.friction * normalized_blend,
            logic: current.logic * (1.0 - normalized_blend) + mood.avec.logic * normalized_blend,
            autonomy: current.autonomy * (1.0 - normalized_blend)
                + mood.avec.autonomy * normalized_blend,
        };

        result.swap_preview = Some(MoodSwapPreview {
            target_mood: mood.name,
            blend: normalized_blend,
            current,
            target: mood.avec,
            blended,
        });

        result
    }
}

fn build_presets() -> Vec<MoodPreset> {
    vec![
        MoodPreset {
            name: "focused".to_string(),
            description: "Deep concentration with low resistance.".to_string(),
            avec: AvecState::focused(),
        },
        MoodPreset {
            name: "creative".to_string(),
            description: "Flexible ideation and exploratory generation.".to_string(),
            avec: AvecState::creative(),
        },
        MoodPreset {
            name: "analytical".to_string(),
            description: "Methodical, precise, high-rigor reasoning.".to_string(),
            avec: AvecState::analytical(),
        },
        MoodPreset {
            name: "exploratory".to_string(),
            description: "Curious search with tolerance for uncertainty.".to_string(),
            avec: AvecState::exploratory(),
        },
        MoodPreset {
            name: "collaborative".to_string(),
            description: "Cooperative and compromise-friendly reasoning.".to_string(),
            avec: AvecState::collaborative(),
        },
        MoodPreset {
            name: "defensive".to_string(),
            description: "Boundary-protective, skeptical stance.".to_string(),
            avec: AvecState::defensive(),
        },
        MoodPreset {
            name: "passive".to_string(),
            description: "Low-agency, low-resistance follow mode.".to_string(),
            avec: AvecState::passive(),
        },
    ]
}
