use mycelium_base::utils::errors::MappedErrors;

#[derive(Clone, Debug)]
pub(crate) struct DepthTracker {
    current_depth: usize,
    max_depth: usize,
}

impl DepthTracker {
    pub(crate) fn new(max_depth: usize) -> Self {
        Self {
            current_depth: 0,
            max_depth,
        }
    }

    pub(crate) fn increment(&mut self) {
        self.current_depth += 1;
    }

    pub(crate) fn should_stop(&self) -> bool {
        self.current_depth >= self.max_depth
    }

    pub(crate) fn empty_value(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

pub(crate) trait ReferenceResolver {
    fn resolve_ref(
        &self,
        components: &serde_json::Value,
        depth_tracker: &mut DepthTracker,
    ) -> Result<serde_json::Value, MappedErrors>;
}
