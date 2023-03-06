#[cfg(test)]
mod tests {
    use cmop::{
        gather_mods, get_mods_from_rules, parse_rules, read_file_as_list, topo_sort, Rules,
    };

    #[test]
    fn test_cycle() {
        let rules = Rules {
            order: vec![
                ("a".to_owned(), "b".into()),
                ("b".into(), "c".into()),
                ("d".into(), "e".into()),
                ("b".into(), "a".into()),
            ],
        };

        let mods = vec!["a", "b", "c", "d", "e"];

        assert!(
            topo_sort(mods, &rules).is_err(),
            "rules do not contain a cycle"
        )
    }

    #[test]
    fn test_ordering() {
        let rules = Rules {
            order: vec![
                ("a".to_owned(), "b".into()),
                ("b".into(), "c".into()),
                ("d".into(), "e".into()),
                ("e".into(), "c".into()),
            ],
        };

        let mods = vec!["a", "b", "c", "d", "e"];

        match topo_sort(mods, &rules) {
            Ok(result) => assert!(checkresult(result, &rules), "order is wrong"),
            Err(_) => panic!("rules contain a cycle"),
        }
    }

    fn checkresult(result: Vec<String>, rules: &Rules) -> bool {
        let pairs = &rules.order;
        for (a, b) in pairs {
            let pos_a = result.iter().position(|x| x == a).unwrap();
            let pos_b = result.iter().position(|x| x == b).unwrap();

            if pos_a > pos_b {
                return false;
            }
        }

        true
    }

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

        assert!(topo_sort(mods, &rules).is_ok(), "rules contain a cycle")
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
