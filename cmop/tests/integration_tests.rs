#[cfg(test)]
mod unit_tests {
    use cmop::{gather_mods, get_mods_from_rules, parse_rules, read_file_as_list, topo_sort};

    #[test]
    fn test_read_mods() {
        let mods_path = "./tests/modlist.txt";
        assert_eq!(read_file_as_list(mods_path), vec!["a", "b", "c", "d", "e"])
    }

    #[test]
    fn test_parse_rules() {
        let rules_path = "./tests/cmop";
        assert!(parse_rules(rules_path).is_ok(), "rules parsing failed")
    }

    #[test]
    fn test_verify_rules() {
        let rules_path = "./tests/cmop";
        let rules = parse_rules(rules_path).unwrap();

        let mods = get_mods_from_rules(&rules);

        assert!(topo_sort(&mods, &rules).is_ok(), "rules contain a cycle")
    }

    #[test]
    fn test_gather_mods() {
        let root_path = "./tests";

        match gather_mods(&root_path) {
            Ok(mods) => {
                assert_eq!(
                    mods,
                    vec![
                        "a.archive".to_owned(),
                        "b.archive".into(),
                        "c.archive".into()
                    ]
                )
            }
            Err(_) => panic!("gethering mods failed"),
        }
    }
}
