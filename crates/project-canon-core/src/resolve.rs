//! Resolution: turn a [`Questionnaire`] into a [`Resolution`] — the base ∪ profile section-set,
//! each dimension stamped `Applies`/`NotApplicable` per its applicability rule.
//!
//! Membership is the union of the base canon and the chosen archetype's profile;
//! **applicability** is a per-dimension status on top. An out-of-scope conditional is
//! `NotApplicable`, never a failure (cli-canon's "n/a, never a fail" rule).

use std::collections::BTreeSet;

use crate::dimension::{Applicability, Archetype, Dimension, Layer};
use crate::profile::Model;
use crate::questionnaire::{Question, Questionnaire};

/// Whether a dimension is in scope for a resolved repo.
///
/// This is the *evaluation-scope* axis, distinct from severity: an `Applies` dimension is in
/// scope to be checked, but whether a failure blocks readiness is decided by its
/// [`Severity`](crate::dimension::Severity) (a `Should` dimension applies yet never gates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppStatus {
    /// In scope for evaluation. (Whether a violation blocks readiness depends on severity.)
    Applies,
    /// Out of scope for this repo — a conditional whose gating question was not answered `true`.
    /// Never a failure. Carries the question that gated it off so a downstream `doctor`/`review`
    /// can report "§20 n/a: Q6 = no" without re-deriving applicability.
    NotApplicable { gated_by: Question },
}

/// The chosen surface shape for §6, selected by Q1. §6 always applies; Q1 only picks its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceShape {
    /// Resource-first noun-verb surface (`$TOOL job create`) — for multi-resource tools.
    NounVerb,
    /// Flat verb-first surface (`cargo build`) — for a genuinely single-resource tool.
    FlatVerb,
}

/// One resolved dimension: which layer it came from and whether it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedDimension {
    /// The dimension's stable id.
    pub id: &'static str,
    /// The layer it was contributed by (base or the profile).
    pub layer: Layer,
    /// Whether it is in scope for this repo.
    pub status: AppStatus,
}

/// The result of resolving a questionnaire against the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    archetype: Archetype,
    surface_shape: Option<SurfaceShape>,
    entries: Vec<ResolvedDimension>,
}

impl Resolution {
    /// The archetype this resolution is for.
    pub fn archetype(&self) -> Archetype {
        self.archetype
    }

    /// The §6 surface shape selected by Q1, or `None` when §6 is not in this resolution's
    /// section-set (a non-CLI archetype has no CLI surface shape — reporting `FlatVerb` for a
    /// `service`/`library` would be a lie).
    pub fn surface_shape(&self) -> Option<SurfaceShape> {
        self.surface_shape
    }

    /// Every resolved dimension (the section-set membership), ordered by id.
    pub fn entries(&self) -> &[ResolvedDimension] {
        &self.entries
    }

    /// The ids of the dimensions that actually apply (status `Applies`).
    pub fn applicable_ids(&self) -> Vec<&'static str> {
        self.entries
            .iter()
            .filter(|e| e.status == AppStatus::Applies)
            .map(|e| e.id)
            .collect()
    }

    /// The canon §N numbers in the section-set membership (regardless of applicability).
    pub fn canon_section_set(&self, model: &Model) -> Vec<u8> {
        let mut sections: Vec<u8> = self
            .entries
            .iter()
            .filter_map(|e| model.dimension(e.id).and_then(Dimension::canon_section))
            .collect();
        sections.sort_unstable();
        sections.dedup();
        sections
    }

    /// The canon §N numbers that actually apply for this repo.
    pub fn applicable_canon_sections(&self, model: &Model) -> Vec<u8> {
        let mut sections: Vec<u8> = self
            .entries
            .iter()
            .filter(|e| e.status == AppStatus::Applies)
            .filter_map(|e| model.dimension(e.id).and_then(Dimension::canon_section))
            .collect();
        sections.sort_unstable();
        sections.dedup();
        sections
    }
}

impl Model {
    /// Resolve `questionnaire` against this model: union base + the chosen profile, stamp each
    /// dimension `Applies`/`NotApplicable`, and select the §6 surface shape from Q1.
    pub fn resolve(&self, questionnaire: &Questionnaire) -> Resolution {
        let archetype = questionnaire.archetype();

        let mut seen: BTreeSet<&'static str> = BTreeSet::new();
        let mut entries: Vec<ResolvedDimension> = Vec::new();

        for id in self.member_ids_for(archetype) {
            // Defensive: base and profile membership are disjoint by construction (see the
            // `debug_assert` in `Model::standard`), so this never fires today. It guards a future
            // author who lets a profile cite a base dimension from double-counting it.
            if !seen.insert(id) {
                continue;
            }
            let dim = self
                .dimension(id)
                .expect("member ids always resolve in the registry");
            let status = match dim.applicability {
                Applicability::Always => AppStatus::Applies,
                Applicability::Conditional(q) => {
                    if questionnaire.answer(q) {
                        AppStatus::Applies
                    } else {
                        AppStatus::NotApplicable { gated_by: q }
                    }
                }
            };
            entries.push(ResolvedDimension {
                id: dim.id,
                layer: dim.layer,
                status,
            });
        }
        entries.sort_by_key(|e| e.id);

        // §6 shape (noun-verb vs. flat, from Q1) is meaningful only when §6 is actually in the
        // resolved set. Non-CLI archetypes have no §6 and thus no surface shape.
        let surface_shape = entries
            .iter()
            .any(|e| self.dimension(e.id).and_then(Dimension::canon_section) == Some(6))
            .then(|| {
                if questionnaire.answer(Question::Q1MultiResource) {
                    SurfaceShape::NounVerb
                } else {
                    SurfaceShape::FlatVerb
                }
            });

        Resolution {
            archetype,
            surface_shape,
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::questionnaire::{Question, Questionnaire};

    #[test]
    fn cli_all_conditionals_yes_applies_all_of_1_to_23() {
        let model = Model::standard();
        let q = Questionnaire::builder(Archetype::Cli)
            .all_conditionals_yes()
            .build();
        let resolution = model.resolve(&q);

        assert_eq!(
            resolution.canon_section_set(&model),
            (1u8..=23).collect::<Vec<_>>()
        );
        assert_eq!(
            resolution.applicable_canon_sections(&model),
            (1u8..=23).collect::<Vec<_>>(),
            "every §1–§23 applies when all conditionals are yes"
        );
    }

    #[test]
    fn cli_with_no_conditionals_marks_conditional_sections_na_but_keeps_membership() {
        let model = Model::standard();
        let q = Questionnaire::builder(Archetype::Cli).build(); // all conditionals false
        let resolution = model.resolve(&q);

        // Membership is still the full §1–§23.
        assert_eq!(
            resolution.canon_section_set(&model),
            (1u8..=23).collect::<Vec<_>>()
        );

        // The conditional sections (§8, §11, §12, §13, §19, §20, §21) are n/a.
        let applies = resolution.applicable_canon_sections(&model);
        for na_section in [8u8, 11, 12, 13, 19, 20, 21] {
            assert!(
                !applies.contains(&na_section),
                "§{na_section} must be n/a with its trigger off"
            );
        }
        // The always-on sections still apply.
        for on in [1u8, 2, 3, 4, 5, 6, 7, 9, 10, 14, 15, 16, 17, 18, 22] {
            assert!(applies.contains(&on), "§{on} must always apply");
        }
    }

    #[test]
    fn empty_profile_resolves_to_base_canon_only() {
        let model = Model::standard();
        let q = Questionnaire::builder(Archetype::Service).build();
        let resolution = model.resolve(&q);

        // Only base dimensions are present — every entry is a base-layer dim.
        assert!(resolution.entries().iter().all(|e| e.layer == Layer::Base));
        // Canon coverage equals the base-canon sections (§10, §15, §16, §17, §22, §23).
        assert_eq!(
            resolution.canon_section_set(&model),
            vec![10u8, 15, 16, 17, 22, 23]
        );
    }

    #[test]
    fn one_conditional_toggles_exactly_its_section() {
        let model = Model::standard();
        let q = Questionnaire::builder(Archetype::Cli)
            .answer(Question::Q6Records, true) // §20 only
            .build();
        let applies = model.resolve(&q).applicable_canon_sections(&model);
        assert!(applies.contains(&20), "§20 on when Q6=yes");
        assert!(!applies.contains(&11), "§11 stays off (Q3=no)");
        assert!(!applies.contains(&19), "§19 stays off (Q5=no)");
    }

    #[test]
    fn q1_selects_surface_shape_without_toggling_section_6() {
        let model = Model::standard();
        let flat = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        assert_eq!(flat.surface_shape(), Some(SurfaceShape::FlatVerb));
        assert!(flat.applicable_canon_sections(&model).contains(&6));

        let noun = model.resolve(
            &Questionnaire::builder(Archetype::Cli)
                .answer(Question::Q1MultiResource, true)
                .build(),
        );
        assert_eq!(noun.surface_shape(), Some(SurfaceShape::NounVerb));
        assert!(noun.applicable_canon_sections(&model).contains(&6));
    }

    #[test]
    fn non_cli_archetype_has_no_surface_shape() {
        let model = Model::standard();
        // §6 is not in a base-only resolution, so there is no CLI surface shape to report.
        for archetype in [Archetype::Service, Archetype::Library, Archetype::Release] {
            let resolution = model.resolve(&Questionnaire::builder(archetype).build());
            assert_eq!(resolution.surface_shape(), None, "{}", archetype.slug());
        }
    }

    #[test]
    fn not_applicable_carries_its_gating_question() {
        let model = Model::standard();
        // Q6 off → §20 is n/a, gated by Q6; the resolution records which question gated it.
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let s20 = resolution
            .entries()
            .iter()
            .find(|e| e.id == crate::canon::canon_id(20))
            .expect("§20 is a member of the cli section-set");
        assert_eq!(
            s20.status,
            AppStatus::NotApplicable {
                gated_by: Question::Q6Records
            }
        );
    }

    #[test]
    fn resolution_entries_are_unique() {
        let model = Model::standard();
        let resolution = model.resolve(&Questionnaire::builder(Archetype::Cli).build());
        let mut ids: Vec<_> = resolution.entries().iter().map(|e| e.id).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(before, ids.len(), "resolution entries must be unique");
    }
}
