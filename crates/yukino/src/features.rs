#![deny(unsafe_code)]

use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::manifest::DependencySpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureRule<'a> {
    Normal(&'a str),
    DepActivate(&'a str),
    DepFeature { dep: &'a str, feature: &'a str },
    WeakDepFeature { dep: &'a str, feature: &'a str },
}

pub fn parse_feature_rule(raw: &str) -> FeatureRule<'_> {
    let trimmed = raw.trim();
    if let Some(dep) = trimmed.strip_prefix("dep:") {
        return FeatureRule::DepActivate(dep.trim());
    }
    if let Some((dep, feat)) = trimmed.split_once("?/") {
        return FeatureRule::WeakDepFeature {
            dep: dep.trim(),
            feature: feat.trim(),
        };
    }
    if let Some((dep, feat)) = trimmed.split_once('/') {
        return FeatureRule::DepFeature {
            dep: dep.trim(),
            feature: feat.trim(),
        };
    }
    FeatureRule::Normal(trimmed)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnifiedFeatures {
    pub root_features: BTreeSet<String>,
    pub active_deps: BTreeSet<String>,
    pub dep_features: HashMap<String, BTreeSet<String>>,
}

/// expands and unifies features across project and dependencies
pub fn unify_dependency_features<S: std::hash::BuildHasher>(
    feature_defs: &HashMap<String, Vec<String>, S>,
    requested: &[String],
    all_features: bool,
    no_default_features: bool,
    deps: &HashMap<String, DependencySpec>,
) -> UnifiedFeatures {
    let mut root_features = BTreeSet::new();
    let mut queue = VecDeque::new();

    if all_features {
        for key in feature_defs.keys() {
            queue.push_back(key.clone());
        }
    }
    if !no_default_features && feature_defs.contains_key("default") {
        queue.push_back("default".to_string());
    }
    for req in requested {
        let trimmed = req.trim();
        if !trimmed.is_empty() {
            queue.push_back(trimmed.to_string());
        }
    }

    let raw_expanded_rules = expand_root_features(feature_defs, &mut queue, &mut root_features);
    let active_deps = resolve_active_deps(deps, &root_features, &raw_expanded_rules);
    let dep_features = collect_dep_features(deps, &active_deps, &raw_expanded_rules);

    UnifiedFeatures {
        root_features,
        active_deps,
        dep_features,
    }
}

fn expand_root_features<S: std::hash::BuildHasher>(
    feature_defs: &HashMap<String, Vec<String>, S>,
    queue: &mut VecDeque<String>,
    root_features: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut rules = Vec::new();
    while let Some(feat) = queue.pop_front() {
        if root_features.contains(&feat) {
            continue;
        }
        root_features.insert(feat.clone());

        if let Some(sub_items) = feature_defs.get(&feat) {
            for sub in sub_items {
                rules.push(sub.clone());
                if let FeatureRule::Normal(name) = parse_feature_rule(sub) {
                    if !name.is_empty() && !root_features.contains(name) {
                        queue.push_back(name.to_string());
                    }
                }
            }
        }
    }
    rules
}

fn resolve_active_deps(
    deps: &HashMap<String, DependencySpec>,
    root_features: &BTreeSet<String>,
    rules: &[String],
) -> BTreeSet<String> {
    let mut active = BTreeSet::new();
    for (dep_name, spec) in deps {
        if !spec.is_optional() {
            active.insert(dep_name.clone());
            continue;
        }
        let mut enabled = root_features.contains(dep_name);
        if !enabled {
            for r in rules {
                match parse_feature_rule(r) {
                    FeatureRule::DepActivate(d) | FeatureRule::DepFeature { dep: d, .. } if d == dep_name => {
                        enabled = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
        if enabled {
            active.insert(dep_name.clone());
        }
    }
    active
}

fn collect_dep_features(
    deps: &HashMap<String, DependencySpec>,
    active_deps: &BTreeSet<String>,
    rules: &[String],
) -> HashMap<String, BTreeSet<String>> {
    let mut map = HashMap::new();
    for dep_name in active_deps {
        let mut set = BTreeSet::new();
        if let Some(spec) = deps.get(dep_name) {
            for f in &spec.features {
                set.insert(f.clone());
            }
            if spec.default_features != Some(false) {
                set.insert("default".to_string());
            }
        }
        map.insert(dep_name.clone(), set);
    }

    for r in rules {
        match parse_feature_rule(r) {
            FeatureRule::DepFeature { dep, feature } | FeatureRule::WeakDepFeature { dep, feature } => {
                if active_deps.contains(dep) && !feature.is_empty() {
                    map.entry(dep.to_string()).or_default().insert(feature.to_string());
                }
            }
            _ => {}
        }
    }
    map
}

/// recur expands features based on feature definition map
pub fn resolve_features<S: std::hash::BuildHasher>(
    feature_map: &HashMap<String, Vec<String>, S>,
    requested: &[String],
    include_default: bool,
) -> BTreeSet<String> {
    let mut resolved = BTreeSet::new();
    let mut queue = VecDeque::new();

    if include_default && feature_map.contains_key("default") {
        queue.push_back("default".to_string());
    }

    for req in requested {
        let trimmed = req.trim();
        if !trimmed.is_empty() {
            queue.push_back(trimmed.to_string());
        }
    }

    while let Some(feat) = queue.pop_front() {
        if resolved.contains(&feat) {
            continue;
        }
        resolved.insert(feat.clone());

        if let Some(sub_features) = feature_map.get(&feat) {
            for sub in sub_features {
                let sub_trimmed = sub.trim();
                if !sub_trimmed.is_empty() && !resolved.contains(sub_trimmed) {
                    queue.push_back(sub_trimmed.to_string());
                }
            }
        }
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_default_features() {
        let mut map = HashMap::new();
        map.insert(
            "default".to_string(),
            vec!["test".to_string(), "test2".to_string()],
        );
        map.insert("test".to_string(), vec![]);
        map.insert("test2".to_string(), vec!["sub_a".to_string()]);
        map.insert("sub_a".to_string(), vec![]);

        let resolved = resolve_features(&map, &[], true);
        assert!(resolved.contains("default"));
        assert!(resolved.contains("test"));
        assert!(resolved.contains("test2"));
        assert!(resolved.contains("sub_a"));
    }

    #[test]
    fn test_resolve_empty_feature() {
        let mut map = HashMap::new();
        map.insert("test3".to_string(), vec![]);

        let resolved = resolve_features(&map, &["test3".to_string()], false);
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains("test3"));
    }

    #[test]
    fn test_resolve_cycle_handling() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), vec!["b".to_string()]);
        map.insert("b".to_string(), vec!["a".to_string()]);

        let resolved = resolve_features(&map, &["a".to_string()], false);
        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains("a"));
        assert!(resolved.contains("b"));
    }

    #[test]
    fn test_resolve_no_default_features() {
        let mut map = HashMap::new();
        map.insert("default".to_string(), vec!["base".to_string()]);
        map.insert("extra".to_string(), vec![]);

        let resolved = resolve_features(&map, &["extra".to_string()], false);
        assert!(!resolved.contains("default"));
        assert!(!resolved.contains("base"));
        assert!(resolved.contains("extra"));
    }

    #[test]
    fn test_parse_feature_rule() {
        assert_eq!(parse_feature_rule("foo"), FeatureRule::Normal("foo"));
        assert_eq!(parse_feature_rule("dep:sqlite"), FeatureRule::DepActivate("sqlite"));
        assert_eq!(
            parse_feature_rule("sqlite/bundled"),
            FeatureRule::DepFeature {
                dep: "sqlite",
                feature: "bundled"
            }
        );
        assert_eq!(
            parse_feature_rule("sqlite?/bundled"),
            FeatureRule::WeakDepFeature {
                dep: "sqlite",
                feature: "bundled"
            }
        );
    }

    #[test]
    fn test_unify_optional_dependency_activation() {
        let mut defs = HashMap::new();
        defs.insert("default".to_string(), vec![]);
        defs.insert("db".to_string(), vec!["dep:sqlite".to_string()]);
        defs.insert("fast_db".to_string(), vec!["sqlite/fast".to_string()]);
        defs.insert("maybe_fast".to_string(), vec!["sqlite?/fast".to_string()]);

        let mut deps = HashMap::new();
        deps.insert(
            "sqlite".to_string(),
            DependencySpec {
                optional: Some(true),
                features: vec!["core".to_string()],
                default_features: Some(true),
                ..Default::default()
            },
        );

        // 1. without requested features -> optional sqlite is inactive
        let u1 = unify_dependency_features(&defs, &[], false, false, &deps);
        assert!(!u1.active_deps.contains("sqlite"));

        // 2. requested "db" -> activates sqlite via dep:sqlite
        let u2 = unify_dependency_features(&defs, &["db".to_string()], false, false, &deps);
        assert!(u2.active_deps.contains("sqlite"));
        let f2 = u2.dep_features.get("sqlite").unwrap();
        assert!(f2.contains("core"));
        assert!(f2.contains("default"));
        assert!(!f2.contains("fast"));

        // 3. requested "fast_db" -> activates sqlite and forwards "fast"
        let u3 = unify_dependency_features(&defs, &["fast_db".to_string()], false, false, &deps);
        assert!(u3.active_deps.contains("sqlite"));
        let f3 = u3.dep_features.get("sqlite").unwrap();
        assert!(f3.contains("fast"));

        // 4. requested "maybe_fast" when sqlite is not otherwise enabled -> weak does not activate
        let u4 = unify_dependency_features(&defs, &["maybe_fast".to_string()], false, false, &deps);
        assert!(!u4.active_deps.contains("sqlite"));

        // 5. requested "db" AND "maybe_fast" -> sqlite active, so "fast" is forwarded
        let u5 = unify_dependency_features(
            &defs,
            &["db".to_string(), "maybe_fast".to_string()],
            false,
            false,
            &deps,
        );
        assert!(u5.active_deps.contains("sqlite"));
        let f5 = u5.dep_features.get("sqlite").unwrap();
        assert!(f5.contains("fast"));
    }

    #[test]
    fn test_unify_all_features_flag() {
        let mut defs = HashMap::new();
        defs.insert("feat1".to_string(), vec![]);
        defs.insert("feat2".to_string(), vec![]);
        defs.insert("default".to_string(), vec!["feat1".to_string()]);

        let deps = HashMap::new();
        let u = unify_dependency_features(&defs, &[], true, false, &deps);
        assert!(u.root_features.contains("feat1"));
        assert!(u.root_features.contains("feat2"));
        assert!(u.root_features.contains("default"));
    }
}
