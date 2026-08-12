//! Routing dimension-discovery candidates to the base layer vs. an archetype profile.
//!
//! The canon grows (cli-canon §19+). When a new dimension-discovery candidate clears the
//! ≥2-tool recurrence bar, it must be routed to a layer. This module encodes the routing rule
//! from the design doc so the decision is mechanical and testable, not ad hoc. It decides
//! *layer only* — admission (the recurrence bar) is upstream.

use crate::dimension::{Archetype, Layer};

/// How broadly a candidate practice applies across archetypes — the input to the routing rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Breadth {
    /// Holds for every archetype (a repo-general practice: doc pattern, changelog, MSRV, git).
    EveryArchetype,
    /// Specific to a single archetype's surface (a CLI-only verb contract, a service-only
    /// health endpoint, a library-only semver-API gate).
    SingleArchetype(Archetype),
    /// Recurs across some but not all archetypes.
    SomeArchetypes(Vec<Archetype>),
}

/// Where a candidate of the given `breadth` should be rooted.
///
/// - Every archetype → [`Layer::Base`].
/// - A single archetype → [`Layer::Profile`] of that archetype.
/// - Several-but-not-all → base, with the non-applicable archetypes gating it off via a
///   `Conditional` applicability (same mechanism as the eight questions). Modelled here as
///   `Base`; the gating question is authored alongside the candidate.
pub fn suggested_layer(breadth: &Breadth) -> Layer {
    match breadth {
        Breadth::EveryArchetype => Layer::Base,
        Breadth::SingleArchetype(a) => Layer::Profile(*a),
        Breadth::SomeArchetypes(_) => Layer::Base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_general_candidate_routes_to_base() {
        assert_eq!(suggested_layer(&Breadth::EveryArchetype), Layer::Base);
    }

    #[test]
    fn archetype_specific_candidate_routes_to_that_profile() {
        assert_eq!(
            suggested_layer(&Breadth::SingleArchetype(Archetype::Cli)),
            Layer::Profile(Archetype::Cli)
        );
        assert_eq!(
            suggested_layer(&Breadth::SingleArchetype(Archetype::Service)),
            Layer::Profile(Archetype::Service)
        );
    }

    #[test]
    fn partial_recurrence_routes_to_base_for_conditional_gating() {
        let breadth = Breadth::SomeArchetypes(vec![Archetype::Cli, Archetype::Service]);
        assert_eq!(suggested_layer(&breadth), Layer::Base);
    }
}
