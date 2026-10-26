//! A studio whose every answer was decided in advance.
//!
//! Scripted rather than clever, and it **counts calls** — which is how the
//! cache and the ceiling get asserted at all: a design that was not paid for is
//! a design the studio was never asked for.

use std::cell::RefCell;

use scorsese_providers::video::ProviderError;
use scorsese_providers::voices::clone::{CloneError, Made, Request};
use scorsese_providers::voices::design::{Brief, Candidate, DesignError, Studio};
use scorsese_providers::voices::{Voice, VoiceError};

/// What one candidate was kept as.
pub(crate) struct Keeping {
    pub(crate) chosen: String,
    pub(crate) name: String,
    pub(crate) passed_over: Vec<String>,
}

/// A studio that answers exactly what it was told to.
pub(crate) struct Fake {
    /// What a design answers with.
    pub(crate) candidates: Vec<Candidate>,
    /// Set to refuse the way a free account is refused.
    pub(crate) free_plan: bool,
    /// Set to refuse the way an unreachable provider does.
    pub(crate) unreachable: bool,
    /// How many times a design was actually asked for.
    pub(crate) designs: RefCell<usize>,
    /// Every candidate that was kept, in order.
    pub(crate) kept: RefCell<Vec<Keeping>>,
    /// Every voice deleted from the account, in order.
    pub(crate) forgotten: RefCell<Vec<String>>,
    /// Every clone asked for, in order.
    pub(crate) cloned: RefCell<Vec<Request>>,
}

impl Fake {
    /// One that designs three candidates with these ids.
    pub(crate) fn offering(ids: &[&str]) -> Self {
        Self {
            candidates: ids
                .iter()
                .enumerate()
                .map(|(index, id)| Candidate {
                    generated_voice_id: (*id).to_owned(),
                    // Not a real MP3 and it does not need to be: nothing under
                    // test decodes one, and a fixture that was real audio would
                    // be a fixture somebody had to keep.
                    sample: format!("sample {index}").into_bytes(),
                    seconds: Some(3.5),
                })
                .collect(),
            free_plan: false,
            unreachable: false,
            designs: RefCell::new(0),
            kept: RefCell::new(Vec::new()),
            forgotten: RefCell::new(Vec::new()),
            cloned: RefCell::new(Vec::new()),
        }
    }

    /// How many times the network would have been touched, and billed.
    pub(crate) fn designs(&self) -> usize {
        *self.designs.borrow()
    }
}

impl Studio for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn candidates(&self, _brief: &Brief) -> Result<Vec<Candidate>, DesignError> {
        *self.designs.borrow_mut() += 1;
        if self.free_plan {
            return Err(DesignError::NotOnThisPlan {
                said: String::from("Voice design is not available on your plan."),
            });
        }
        if self.unreachable {
            return Err(DesignError::Voice(VoiceError::Provider(
                ProviderError::new("fake", "no route to host"),
            )));
        }
        Ok(self.candidates.clone())
    }

    fn keep(
        &self,
        _brief: &Brief,
        chosen: &str,
        name: &str,
        passed_over: &[String],
    ) -> Result<Voice, DesignError> {
        self.kept.borrow_mut().push(Keeping {
            chosen: chosen.to_owned(),
            name: name.to_owned(),
            passed_over: passed_over.to_vec(),
        });
        Ok(Voice {
            id: format!("voice-for-{chosen}"),
            name: name.to_owned(),
            traits: vec![String::from("designed")],
            description: None,
            preview: None,
        })
    }

    fn forget(&self, voice_id: &str) -> Result<(), DesignError> {
        if self.unreachable {
            return Err(DesignError::Voice(VoiceError::Provider(
                ProviderError::new("fake", "no route to host"),
            )));
        }
        self.forgotten.borrow_mut().push(voice_id.to_owned());
        Ok(())
    }

    fn clone_voice(&self, request: &Request) -> Result<Made, CloneError> {
        if self.free_plan {
            return Err(CloneError::NotOnThisPlan {
                said: String::from("Instant voice cloning is not available on your plan."),
            });
        }
        self.cloned.borrow_mut().push(request.clone());
        Ok(Made {
            voice: Voice {
                id: format!("clone-{}", self.cloned.borrow().len()),
                name: request.name.clone(),
                traits: vec![String::from("cloned")],
                description: request.description.clone(),
                preview: None,
            },
            requires_verification: false,
        })
    }
}
