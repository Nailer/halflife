//! Impact projection — *what would break if this dependency became unsafe?*
//!
//! Pure logic. The registry enters through [`RegistrySource`] rather than being
//! read directly, so projection stays fast, testable, and usable outside a
//! demo: the same code answers a CLI query, a CI check, and an API request.
//!
//! # Disclosure is a query parameter, not a filter
//!
//! The single most important property here. A public projection is computed
//! **from authorized evidence only** — it is never built in full and then
//! filtered down. Post-filtering leaks: the counts change, the response shape
//! changes, and an observer learns that something was removed. Passing
//! [`Audience`] into the source means the withheld rows are never assembled.
//!
//! The observable consequence, which is deliberate: to a public caller, a
//! dependency used only by embargoed circuits is **indistinguishable** from one
//! nothing uses at all. See `projection_of_an_embargoed_only_dependency_is_indistinguishable`.

use anyhow::Result;
use halflife_core::Audience;
use serde::Serialize;

/// One circuit reached by a dependency query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reached {
    pub circuit_hash: String,
    pub name: String,
    pub repository: String,
    /// Advisories matched against *this dependency* in *this circuit's* closure.
    pub advisories: Vec<String>,
}

impl Reached {
    pub fn is_affected(&self) -> bool {
        !self.advisories.is_empty()
    }
}

/// The registry, as projection needs it. Implemented over SQLite in
/// `halflife-registry`, and over a vector in tests.
pub trait RegistrySource {
    /// Circuits whose resolved closure contains exactly `name@version`,
    /// restricted to what `audience` is authorized to see.
    fn reached_by(&self, name: &str, version: &str, audience: Audience)
        -> Result<Vec<Reached>>;

    /// How many circuits `audience` can see at all. Used for the
    /// *not reached* figure, and must exclude what they may not see.
    fn visible_circuits(&self, audience: Audience) -> Result<usize>;
}

#[derive(Debug, Clone, Serialize)]
pub struct Impact {
    pub dependency: String,
    pub version: String,
    pub audience: Audience,
    /// Reached and carrying at least one advisory.
    pub affected: Vec<Reached>,
    /// Reached, but with nothing matched against it.
    pub clean: Vec<Reached>,
    /// Visible circuits whose closure does not contain this dependency.
    pub not_reached: usize,
    /// Every distinct advisory implicated across the affected set.
    pub advisories: Vec<String>,
}

impl Impact {
    pub fn reached(&self) -> usize {
        self.affected.len() + self.clean.len()
    }
    pub fn is_clear(&self) -> bool {
        self.affected.is_empty()
    }
}

/// Project the impact of a dependency over the registered graph.
pub fn project<S: RegistrySource + ?Sized>(
    source: &S,
    name: &str,
    version: &str,
    audience: Audience,
) -> Result<Impact> {
    let reached = source.reached_by(name, version, audience)?;
    let visible = source.visible_circuits(audience)?;

    let (affected, clean): (Vec<_>, Vec<_>) =
        reached.into_iter().partition(Reached::is_affected);

    let mut advisories: Vec<String> = affected
        .iter()
        .flat_map(|r| r.advisories.iter().cloned())
        .collect();
    advisories.sort();
    advisories.dedup();

    Ok(Impact {
        dependency: name.to_string(),
        version: version.to_string(),
        audience,
        not_reached: visible.saturating_sub(affected.len() + clean.len()),
        affected,
        clean,
        advisories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory source. `public` marks whether a circuit's finding is
    /// disclosable; the fake enforces the same rule the SQL does.
    struct Fake {
        rows: Vec<(Reached, bool)>,
    }

    impl Fake {
        fn new(rows: Vec<(&str, &str, Vec<&str>, bool)>) -> Self {
            Self {
                rows: rows
                    .into_iter()
                    .map(|(hash, name, advs, public)| {
                        (
                            Reached {
                                circuit_hash: hash.into(),
                                name: name.into(),
                                repository: format!("https://example.test/{name}"),
                                advisories: advs.into_iter().map(String::from).collect(),
                            },
                            public,
                        )
                    })
                    .collect(),
            }
        }
    }

    // Deliberately simplistic: every circuit here holds the same single
    // dependency, so `reached_by` is an authorization test, not a graph test.
    impl RegistrySource for Fake {
        fn reached_by(&self, name: &str, _v: &str, audience: Audience) -> Result<Vec<Reached>> {
            Ok(self
                .rows
                .iter()
                .filter(|(_, public)| *public || !audience.is_public())
                .filter(|(r, _)| r.name.contains(name) || name == "dep")
                .map(|(r, _)| r.clone())
                .collect())
        }
        fn visible_circuits(&self, audience: Audience) -> Result<usize> {
            Ok(self
                .rows
                .iter()
                .filter(|(_, public)| *public || !audience.is_public())
                .count())
        }
    }

    fn fleet() -> Fake {
        Fake::new(vec![
            ("aaa", "affected-one", vec!["GHSA-ww9q-8r59-xv46"], true),
            ("bbb", "affected-two", vec!["GHSA-ww9q-8r59-xv46"], true),
            ("ccc", "clean-one", vec![], true),
        ])
    }

    #[test]
    fn affected_and_clean_are_partitioned() {
        let i = project(&fleet(), "dep", "0.4.0", Audience::Operator).unwrap();
        assert_eq!(i.affected.len(), 2);
        assert_eq!(i.clean.len(), 1);
        assert_eq!(i.reached(), 3);
        assert_eq!(i.advisories, vec!["GHSA-ww9q-8r59-xv46"]);
        assert!(!i.is_clear());
    }

    #[test]
    fn a_healthy_dependency_produces_no_false_impact() {
        // The step-10 property: reaching circuits is not the same as affecting
        // them. Anyone can build an alarm; discrimination is the claim.
        let f = Fake::new(vec![
            ("ccc", "clean-one", vec![], true),
            ("ddd", "clean-two", vec![], true),
        ]);
        let i = project(&f, "dep", "0.5.0", Audience::Public).unwrap();
        assert_eq!(i.reached(), 2);
        assert!(i.affected.is_empty());
        assert!(i.is_clear());
        assert!(i.advisories.is_empty());
    }

    #[test]
    fn embargoed_circuits_are_absent_from_a_public_projection() {
        let f = Fake::new(vec![
            ("aaa", "public-affected", vec!["GHSA-ww9q-8r59-xv46"], true),
            ("sss", "secret-affected", vec!["GHSA-ww9q-8r59-xv46"], false),
        ]);
        let pubv = project(&f, "dep", "0.4.0", Audience::Public).unwrap();
        assert_eq!(pubv.affected.len(), 1);
        assert_eq!(pubv.affected[0].name, "public-affected");

        let op = project(&f, "dep", "0.4.0", Audience::Operator).unwrap();
        assert_eq!(op.affected.len(), 2);
    }

    #[test]
    fn counts_do_not_reveal_withheld_circuits() {
        // If `not_reached` were computed against the true total, a public caller
        // could subtract and learn how many circuits they are not being shown.
        let f = Fake::new(vec![
            ("aaa", "public-affected", vec!["GHSA-ww9q-8r59-xv46"], true),
            ("sss", "secret-one", vec!["GHSA-ww9q-8r59-xv46"], false),
            ("ttt", "secret-two", vec![], false),
        ]);
        let pubv = project(&f, "dep", "0.4.0", Audience::Public).unwrap();
        assert_eq!(pubv.reached(), 1);
        assert_eq!(pubv.not_reached, 0, "withheld circuits must not appear in any count");
    }

    #[test]
    fn projection_of_an_embargoed_only_dependency_is_indistinguishable() {
        // The subtle leak. A dependency used *only* by embargoed circuits must
        // look exactly like a dependency nothing uses. Any difference -- a
        // "restricted" marker, a non-zero count, a distinct error -- confirms
        // that something exists, which is what the embargo is hiding.
        let embargoed_only = Fake::new(vec![
            ("sss", "secret", vec!["GHSA-ww9q-8r59-xv46"], false),
        ]);
        let nothing = Fake::new(vec![]);

        let a = project(&embargoed_only, "dep", "0.4.0", Audience::Public).unwrap();
        let b = project(&nothing, "dep", "0.4.0", Audience::Public).unwrap();

        assert_eq!(a.affected, b.affected);
        assert_eq!(a.clean, b.clean);
        assert_eq!(a.not_reached, b.not_reached);
        assert_eq!(a.advisories, b.advisories);
        assert!(a.is_clear() && b.is_clear());
    }
}
