//! The applicability questionnaire — the *characterize → applicable-sections* mechanism,
//! mirrored from `cli-canon`'s `SKILL.md` "Characterize the tool" step.
//!
//! It does two jobs: (1) select the repo's [`Archetype`] (which profile layers onto base) and
//! (2) gate the conditional sections via the eight yes/no questions lifted verbatim from
//! cli-canon. See [`crate::resolve`] for how the answers become a [`crate::resolve::Resolution`].

use std::collections::BTreeMap;

use crate::dimension::Archetype;

/// The eight yes/no characterization questions. Each maps to the conditional section(s) it
/// gates — the mapping lives on the section's [`crate::dimension::Applicability`], so this enum
/// stays a pure description of *what was asked*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Question {
    /// Q1 — More than one resource noun? (shapes §6: noun-verb vs. flat verb surface)
    Q1MultiResource,
    /// Q2 — Resolves persistent config and/or a data root? (gates §8)
    Q2ConfigOrDataRoot,
    /// Q3 — Any command creates/updates/deletes a resource? (gates §11)
    Q3Mutates,
    /// Q4 — Any command runs more than a few seconds / as a daemon? (gates §12)
    Q4LongRunning,
    /// Q5 — Stamps `created`/`updated` or derives time-based ids? (gates §19)
    Q5Timestamps,
    /// Q6 — Owns human-editable on-disk records? (gates §20)
    Q6Records,
    /// Q7 — Scaffolds an on-disk home other commands need? (gates §21)
    Q7Scaffolds,
    /// Q8 — Results that can be large (list/export)? (gates §13)
    Q8LargeResults,
}

impl Question {
    /// All eight questions, in canonical Q1..Q8 order.
    pub const ALL: [Question; 8] = [
        Question::Q1MultiResource,
        Question::Q2ConfigOrDataRoot,
        Question::Q3Mutates,
        Question::Q4LongRunning,
        Question::Q5Timestamps,
        Question::Q6Records,
        Question::Q7Scaffolds,
        Question::Q8LargeResults,
    ];

    /// The `Q1`..`Q8` label.
    pub fn label(self) -> &'static str {
        match self {
            Question::Q1MultiResource => "Q1",
            Question::Q2ConfigOrDataRoot => "Q2",
            Question::Q3Mutates => "Q3",
            Question::Q4LongRunning => "Q4",
            Question::Q5Timestamps => "Q5",
            Question::Q6Records => "Q6",
            Question::Q7Scaffolds => "Q7",
            Question::Q8LargeResults => "Q8",
        }
    }

    /// The full question text (as posed in cli-canon's characterization table).
    pub fn prompt(self) -> &'static str {
        match self {
            Question::Q1MultiResource => "More than one resource noun?",
            Question::Q2ConfigOrDataRoot => "Resolves persistent config and/or a data root?",
            Question::Q3Mutates => "Any command creates/updates/deletes a resource?",
            Question::Q4LongRunning => "Any command runs more than a few seconds / as a daemon?",
            Question::Q5Timestamps => "Stamps created/updated or derives time-based ids?",
            Question::Q6Records => "Owns human-editable on-disk records?",
            Question::Q7Scaffolds => "Scaffolds an on-disk home other commands need?",
            Question::Q8LargeResults => "Results that can be large (list/export)?",
        }
    }
}

/// A repo's answers: the chosen archetype plus the eight conditional answers.
///
/// Unanswered conditionals default to `false` — the conservative choice: an unproven trigger
/// leaves its section out of scope (`n/a`) rather than asserting a requirement the repo may not
/// need. Build one with [`Questionnaire::builder`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Questionnaire {
    archetype: Archetype,
    conditionals: BTreeMap<Question, bool>,
}

impl Questionnaire {
    /// Start a questionnaire for `archetype`. All conditionals default to `false`.
    pub fn builder(archetype: Archetype) -> QuestionnaireBuilder {
        QuestionnaireBuilder {
            archetype,
            conditionals: BTreeMap::new(),
        }
    }

    /// The chosen archetype.
    pub fn archetype(&self) -> Archetype {
        self.archetype
    }

    /// The answer to a conditional question (unanswered ⇒ `false`).
    pub fn answer(&self, question: Question) -> bool {
        self.conditionals.get(&question).copied().unwrap_or(false)
    }
}

/// Builder for a [`Questionnaire`].
#[derive(Debug, Clone)]
pub struct QuestionnaireBuilder {
    archetype: Archetype,
    conditionals: BTreeMap<Question, bool>,
}

impl QuestionnaireBuilder {
    /// Record one conditional answer.
    pub fn answer(mut self, question: Question, value: bool) -> Self {
        self.conditionals.insert(question, value);
        self
    }

    /// Answer every one of the eight conditionals `true` — the maximal surface (a tool that
    /// exercises every conditional section). Handy for tests and for the "widest" scaffold.
    pub fn all_conditionals_yes(mut self) -> Self {
        for q in Question::ALL {
            self.conditionals.insert(q, true);
        }
        self
    }

    /// Finish building.
    pub fn build(self) -> Questionnaire {
        Questionnaire {
            archetype: self.archetype,
            conditionals: self.conditionals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unanswered_conditionals_default_false() {
        let q = Questionnaire::builder(Archetype::Cli).build();
        for question in Question::ALL {
            assert!(
                !q.answer(question),
                "{} should default false",
                question.label()
            );
        }
    }

    #[test]
    fn all_conditionals_yes_sets_every_question() {
        let q = Questionnaire::builder(Archetype::Cli)
            .all_conditionals_yes()
            .build();
        for question in Question::ALL {
            assert!(q.answer(question));
        }
    }

    #[test]
    fn individual_answer_overrides_default() {
        let q = Questionnaire::builder(Archetype::Service)
            .answer(Question::Q2ConfigOrDataRoot, true)
            .build();
        assert!(q.answer(Question::Q2ConfigOrDataRoot));
        assert!(!q.answer(Question::Q3Mutates));
        assert_eq!(q.archetype(), Archetype::Service);
    }
}
