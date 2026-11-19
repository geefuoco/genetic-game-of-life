//! Share links: the Rule, settings and Seed of a Soup World as text.

use genetic_life_core::{MutationSettings, Rule, ShareLinkError, Soup};
use proptest::prelude::*;

fn soup() -> Soup {
    Soup {
        seed: 42,
        rule: Rule::resolve("highlife").unwrap(),
        width: 64,
        height: 48,
        density: 0.3,
        founding_alleles: 4,
        mutation: MutationSettings::new(0.01, 30.0).unwrap(),
    }
}

#[test]
fn a_share_link_is_readable_text() {
    assert_eq!(
        soup().share_link(),
        "v=1&seed=42&rule=B36/S23&width=64&height=48&density=0.3&alleles=4&mutation-rate=0.01&mutation-magnitude=30"
    );
}

#[test]
fn a_share_link_decodes_from_a_full_url() {
    let link = format!("https://example.com/life/#{}", soup().share_link());
    assert_eq!(Soup::from_share_link(&link), Ok(soup()));
}

#[test]
fn a_share_link_decodes_with_its_fields_in_any_order() {
    let link = "seed=42&v=1&mutation-magnitude=30&rule=b36/s23&alleles=4&width=64&density=0.3&height=48&mutation-rate=0.01";
    assert_eq!(Soup::from_share_link(link), Ok(soup()));
}

#[test]
fn a_soup_starts_its_world_with_its_rule_and_settings() {
    let soup = soup();
    let world = soup.world().unwrap();
    assert_eq!((world.width(), world.height()), (64, 48));
    assert_eq!(world.generation(), 0);
    assert_eq!(world.rule(), soup.rule);
    assert_eq!(world.mutation(), soup.mutation);
    assert_eq!(world.census().alleles.len(), 4);
    assert_eq!(soup.world().unwrap(), world);
}

#[test]
fn a_soup_with_invalid_settings_has_no_world() {
    let soup = Soup { width: 8, ..soup() };
    assert!(soup.world().is_err());
}

#[test]
fn malformed_share_links_are_rejected_with_a_clear_error() {
    let valid = soup().share_link();
    let cases = [
        ("empty", String::new()),
        ("no version", valid.replace("v=1&", "")),
        ("unknown version", valid.replace("v=1", "v=2")),
        ("missing field", valid.replace("&alleles=4", "")),
        ("unknown field", format!("{valid}&colour=red")),
        ("duplicate field", format!("{valid}&seed=7")),
        ("pair without '='", format!("{valid}&seed")),
        (
            "seed not a number",
            valid.replace("seed=42", "seed=forty-two"),
        ),
        ("negative seed", valid.replace("seed=42", "seed=-1")),
        ("width not a number", valid.replace("width=64", "width=6.4")),
        (
            "density not a number",
            valid.replace("density=0.3", "density=lots"),
        ),
        ("bad rule", valid.replace("B36/S23", "B9/S23")),
        ("world too large", valid.replace("width=64", "width=2048")),
        (
            "density out of range",
            valid.replace("density=0.3", "density=1.5"),
        ),
        (
            "density not finite",
            valid.replace("density=0.3", "density=NaN"),
        ),
        (
            "too many founding alleles",
            valid.replace("alleles=4", "alleles=17"),
        ),
        (
            "mutation rate out of range",
            valid.replace("mutation-rate=0.01", "mutation-rate=2"),
        ),
        (
            "mutation magnitude out of range",
            valid.replace("mutation-magnitude=30", "mutation-magnitude=181"),
        ),
    ];
    for (case, link) in cases {
        let error = Soup::from_share_link(&link).expect_err(case);
        assert!(!error.to_string().is_empty(), "{case}");
    }
}

#[test]
fn a_share_link_names_what_is_wrong() {
    let valid = soup().share_link();
    assert_eq!(
        Soup::from_share_link(&valid.replace("v=1", "v=2")),
        Err(ShareLinkError::UnsupportedVersion("2".into()))
    );
    assert_eq!(
        Soup::from_share_link(&valid.replace("&alleles=4", "")),
        Err(ShareLinkError::MissingField("alleles"))
    );
    let error = Soup::from_share_link(&valid.replace("seed=42", "seed=x")).unwrap_err();
    assert!(error.to_string().contains("seed"), "{error}");
}

fn rule() -> impl Strategy<Value = Rule> {
    (0u16..512, 0u16..512).prop_map(|(birth, survival)| {
        let digits = |mask: u16| -> String {
            (0..=8)
                .filter(|n| mask >> n & 1 == 1)
                .map(|n| n.to_string())
                .collect()
        };
        format!("B{}/S{}", digits(birth), digits(survival))
            .parse()
            .unwrap()
    })
}

proptest! {
    #[test]
    fn a_share_link_decodes_to_the_rule_settings_and_seed_it_encodes(
        seed in any::<u64>(),
        rule in rule(),
        width in 16usize..=1024,
        height in 16usize..=1024,
        density in 0.0f64..=1.0,
        founding_alleles in 1usize..=16,
        rate in 0.0f64..=1.0,
        magnitude in 0.0f64..=180.0,
    ) {
        let soup = Soup {
            seed,
            rule,
            width,
            height,
            density,
            founding_alleles,
            mutation: MutationSettings::new(rate, magnitude).unwrap(),
        };
        prop_assert_eq!(Soup::from_share_link(&soup.share_link()), Ok(soup));
    }
}
