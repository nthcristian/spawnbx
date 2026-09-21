use super::types::{DesiredState, Observation, Transition};

pub(crate) trait LifecycleModule {
    fn decide(
        &mut self,
        desired: &DesiredState,
        observed: Option<&Observation>,
    ) -> Result<Transition, LifecycleError>;
}

pub(crate) struct LifecycleDecider;

impl LifecycleDecider {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl LifecycleModule for LifecycleDecider {
    fn decide(
        &mut self,
        desired: &DesiredState,
        observed: Option<&Observation>,
    ) -> Result<Transition, LifecycleError> {
        let Some(observed) = observed else {
            return Ok(Transition::Create);
        };
        if !observed.managed {
            return Ok(Transition::Collision);
        }
        if observed.observed_hash.lowercase_hex == desired.hash.lowercase_hex
            && observed.observed_image.digest == desired.container.image.digest
        {
            return Ok(if observed.running {
                Transition::Reuse
            } else {
                Transition::Start
            });
        }
        Ok(Transition::Recreate)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LifecycleError;
