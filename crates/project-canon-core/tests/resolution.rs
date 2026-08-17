//! Done-criteria integration tests for the base-canon + archetype-profile model.
//!
//! These assert the contract the issue's Done criteria name, against the public API only:
//! - the `cli` profile resolves to the §1–§24 section-set;
//! - the base canon resolves to its seeded sections;
//! - `service`/`library`/`release` exist as named-but-empty profiles that resolve empty.

use project_canon_core::{AppStatus, Archetype, Model, Question, Questionnaire};

#[test]
fn cli_profile_resolves_to_sections_1_through_24() {
    let model = Model::standard();
    let q = Questionnaire::builder(Archetype::Cli)
        .all_conditionals_yes()
        .build();
    let resolution = model.resolve(&q);

    // The section-set membership is exactly §1..=§24.
    assert_eq!(
        resolution.canon_section_set(&model),
        (1u8..=24).collect::<Vec<_>>()
    );
    // With every conditional satisfied, all of §1..=§24 apply.
    assert_eq!(
        resolution.applicable_canon_sections(&model),
        (1u8..=24).collect::<Vec<_>>()
    );
}

#[test]
fn base_canon_resolves_to_its_seeded_sections() {
    let model = Model::standard();
    // Seeded repo-general canon sections: §10, §15, §16, §17, §22, §23, §24.
    let q = Questionnaire::builder(Archetype::Service).build();
    let base_only = model.resolve(&q);
    assert_eq!(
        base_only.canon_section_set(&model),
        vec![10u8, 15, 16, 17, 22, 23, 24]
    );

    // Plus the create-project scaffold dims, all Always-applicable.
    let base_ids: Vec<&str> = model.base_members().to_vec();
    for scaffold in [
        "base.doc-pattern",
        "base.issue-tracking",
        "base.git-hygiene",
        "base.readme",
        "base.gitignore",
    ] {
        assert!(base_ids.contains(&scaffold), "{scaffold} missing from base");
        let entry = base_only
            .entries()
            .iter()
            .find(|e| e.id == scaffold)
            .expect("scaffold dim present in a base-only resolution");
        assert_eq!(entry.status, AppStatus::Applies);
    }
}

#[test]
fn service_library_release_are_named_but_empty_extension_points() {
    let model = Model::standard();
    for archetype in [Archetype::Service, Archetype::Library, Archetype::Release] {
        let profile = model.profile(archetype);
        assert!(profile.is_extension_point());
        assert!(profile.members().is_empty());

        // The extension-point contract: resolving one yields base canon and nothing more.
        let resolution = model.resolve(&Questionnaire::builder(archetype).build());
        assert_eq!(
            resolution.canon_section_set(&model),
            vec![10u8, 15, 16, 17, 22, 23, 24],
            "{} must resolve to base canon only",
            archetype.slug()
        );
    }
}

#[test]
fn questionnaire_selects_profile_and_gates_conditionals() {
    let model = Model::standard();
    // A CLI that mutates and owns records but is not long-running and has no large output.
    let q = Questionnaire::builder(Archetype::Cli)
        .answer(Question::Q2ConfigOrDataRoot, true) // §8
        .answer(Question::Q3Mutates, true) // §11
        .answer(Question::Q6Records, true) // §20
        .build();
    let applies = model.resolve(&q).applicable_canon_sections(&model);

    for on in [8u8, 11, 20] {
        assert!(applies.contains(&on), "§{on} should apply");
    }
    for off in [12u8, 13, 19, 21] {
        assert!(!applies.contains(&off), "§{off} should be n/a");
    }
}
