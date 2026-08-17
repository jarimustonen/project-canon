//! The [`Model`]: the base canon, the archetype profiles, and the shared dimension registry.
//!
//! A **profile** is a named section-set + a probe registry. The "probe registry" is not a
//! second structure — a profile's probes are the [`Probe`](crate::dimension::Probe)s of its
//! member dimensions, looked up in the one shared registry. Keeping a single registry means the
//! probe a future `review` runs and the scaffold a future `new` emits can never drift from the
//! section they belong to.

use std::collections::BTreeMap;

use crate::canon::canon_dimensions;
use crate::dimension::{Archetype, Dimension, Layer};
use crate::scaffold::scaffold_dimensions;

/// A profile: the additive section-set one archetype layers on top of the base canon.
///
/// `members` are the profile-*rooted* dimension ids (its own contribution); the base layer
/// contributes the repo-general dimensions independently. An empty `members` is the
/// named-but-empty extension-point state (`service`/`library`/`release` at v0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    archetype: Archetype,
    members: Vec<&'static str>,
}

impl Profile {
    /// The archetype this profile is for.
    pub fn archetype(&self) -> Archetype {
        self.archetype
    }

    /// The profile-rooted dimension ids (its additive contribution over base).
    pub fn members(&self) -> &[&'static str] {
        &self.members
    }

    /// True when this profile adds nothing — the named-but-empty extension-point contract.
    pub fn is_extension_point(&self) -> bool {
        self.members.is_empty()
    }
}

/// The whole v0 conformance model: one dimension registry, the base-canon member set, and the
/// four archetype profiles.
#[derive(Debug, Clone)]
pub struct Model {
    registry: BTreeMap<&'static str, Dimension>,
    base: Vec<&'static str>,
    profiles: BTreeMap<Archetype, Profile>,
}

impl Model {
    /// Build the standard, seeded v0 model:
    ///
    /// - **base canon** = the repo-general canon sections (§10, §15–§17, §22–§23) + the
    ///   create-project scaffold dimensions;
    /// - **`cli` profile** = the 17 CLI-surface canon sections (the §1–§23 sections not rooted
    ///   in base); together with base, `cli` resolves to the full §1–§23;
    /// - **`service` / `library` / `release`** = named-but-empty extension points.
    pub fn standard() -> Self {
        let mut registry: BTreeMap<&'static str, Dimension> = BTreeMap::new();
        for dim in canon_dimensions().into_iter().chain(scaffold_dimensions()) {
            // The id is the stable citation surface; a duplicate would silently drop a
            // dimension. Fail construction loudly instead.
            assert!(
                registry.insert(dim.id, dim).is_none(),
                "duplicate dimension id: {}",
                dim.id
            );
        }

        // Base = every dimension rooted in Layer::Base (both the base canon sections and the
        // scaffold dims). Deriving it from the registry keeps base membership and each
        // dimension's declared layer in lockstep (asserted by test).
        let mut base: Vec<&'static str> = registry
            .values()
            .filter(|d| d.layer == Layer::Base)
            .map(|d| d.id)
            .collect();
        base.sort_unstable();

        // The cli profile's own members: canon sections rooted in Profile(Cli).
        let mut cli_members: Vec<&'static str> = registry
            .values()
            .filter(|d| d.layer == Layer::Profile(Archetype::Cli))
            .map(|d| d.id)
            .collect();
        cli_members.sort_unstable();

        // Base and profile membership are derived from mutually-exclusive `Layer` filters, so a
        // dimension can be rooted in exactly one layer — base ∩ profile is empty by
        // construction. Assert it so a future author who reworks the layering (e.g. to let a
        // profile cite a base dimension) is forced to revisit the dedup in `resolve` rather than
        // silently double-counting.
        debug_assert!(
            cli_members.iter().all(|id| !base.contains(id)),
            "base and cli-profile membership must be disjoint"
        );

        let mut profiles = BTreeMap::new();
        profiles.insert(
            Archetype::Cli,
            Profile {
                archetype: Archetype::Cli,
                members: cli_members,
            },
        );
        // Named-but-empty extension points.
        for archetype in [Archetype::Service, Archetype::Library, Archetype::Release] {
            profiles.insert(
                archetype,
                Profile {
                    archetype,
                    members: Vec::new(),
                },
            );
        }

        Model {
            registry,
            base,
            profiles,
        }
    }

    /// Look up a dimension by its stable id.
    pub fn dimension(&self, id: &str) -> Option<&Dimension> {
        self.registry.get(id)
    }

    /// Every dimension in the registry, ordered by id.
    pub fn dimensions(&self) -> impl Iterator<Item = &Dimension> {
        self.registry.values()
    }

    /// The base-canon member ids (repo-invariant), ordered.
    pub fn base_members(&self) -> &[&'static str] {
        &self.base
    }

    /// The profile for `archetype`. Every archetype has one (empty for the v0 extension points).
    pub fn profile(&self, archetype: Archetype) -> &Profile {
        self.profiles
            .get(&archetype)
            .expect("every Archetype has a profile in the standard model")
    }

    /// The canon §N numbers a resolution of `archetype` would cover — the union of base and the
    /// profile's canon sections. For `cli` this is the full `1..=23`.
    pub fn canon_sections_for(&self, archetype: Archetype) -> Vec<u8> {
        let mut sections: Vec<u8> = self
            .member_ids_for(archetype)
            .filter_map(|id| self.registry.get(id).and_then(Dimension::canon_section))
            .collect();
        sections.sort_unstable();
        sections.dedup();
        sections
    }

    /// The base member ids chained with `archetype`'s profile members — base first, then
    /// profile, each already ordered. Not itself deduplicated: dedup (harmless today, since the
    /// two sets are disjoint by construction) is applied by the callers that need a set
    /// ([`canon_sections_for`](Self::canon_sections_for) and `resolve`).
    pub(crate) fn member_ids_for(
        &self,
        archetype: Archetype,
    ) -> impl Iterator<Item = &'static str> + '_ {
        self.base
            .iter()
            .copied()
            .chain(self.profile(archetype).members().iter().copied())
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::standard()
    }
}

/// The canon sections routed to base — the base-canon citation surface, used to assert the
/// extension-point contract (an empty profile resolves to exactly these).
#[cfg(test)]
pub(crate) fn base_canon_sections() -> Vec<u8> {
    (1u8..=23)
        .filter(|&n| crate::canon::is_base_section(n))
        .collect()
}

/// True if `source` is a canon citation (vs. scaffold/discovered).
#[cfg(test)]
pub(crate) fn is_canon(source: crate::dimension::DimensionSource) -> bool {
    matches!(source, crate::dimension::DimensionSource::Canon { .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_canon_holds_the_seeded_sections_and_scaffold_dims() {
        let model = Model::standard();
        // The repo-general canon sections routed to base.
        for section in [10u8, 15, 16, 17, 22, 23] {
            let id = crate::canon::canon_id(section);
            assert!(
                model.base_members().contains(&id),
                "§{section} should be a base member"
            );
        }
        // The scaffold dims.
        for id in [
            "base.doc-pattern",
            "base.issue-tracking",
            "base.git-hygiene",
            "base.readme",
            "base.gitignore",
        ] {
            assert!(
                model.base_members().contains(&id),
                "{id} should be a base member"
            );
        }
    }

    #[test]
    fn base_never_holds_a_cli_surface_section() {
        let model = Model::standard();
        // §1 is a CLI-surface section, not repo-general.
        assert!(!model.base_members().contains(&crate::canon::canon_id(1)));
    }

    #[test]
    fn cli_profile_plus_base_covers_all_of_1_to_23() {
        let model = Model::standard();
        assert_eq!(
            model.canon_sections_for(Archetype::Cli),
            (1u8..=23).collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_profiles_are_extension_points_and_resolve_to_base_canon_only() {
        let model = Model::standard();
        for archetype in [Archetype::Service, Archetype::Library, Archetype::Release] {
            let profile = model.profile(archetype);
            assert!(
                profile.is_extension_point(),
                "{} should be empty",
                archetype.slug()
            );
            assert!(profile.members().is_empty());
            // An empty profile resolves (its canon coverage) to exactly the base-canon sections.
            assert_eq!(model.canon_sections_for(archetype), base_canon_sections());
        }
    }

    #[test]
    fn every_registry_dimension_has_a_layer_matching_its_membership() {
        let model = Model::standard();
        for dim in model.dimensions() {
            match dim.layer {
                Layer::Base => assert!(model.base_members().contains(&dim.id)),
                Layer::Profile(a) => assert!(model.profile(a).members().contains(&dim.id)),
            }
        }
    }

    #[test]
    fn base_holds_no_conditional_dimensions() {
        // Repo-invariant base dims must be Always-on; a conditional base dim would have no
        // question to gate it consistently across archetypes.
        let model = Model::standard();
        for id in model.base_members() {
            let dim = model.dimension(id).unwrap();
            assert!(
                dim.applicability.question().is_none(),
                "base dim {id} must be Always"
            );
            // Sanity: base members are canon or scaffold, never left unclassified.
            assert!(
                is_canon(dim.source)
                    || matches!(dim.source, crate::dimension::DimensionSource::Scaffold),
                "base dim {id} has an unexpected source"
            );
        }
    }
}
