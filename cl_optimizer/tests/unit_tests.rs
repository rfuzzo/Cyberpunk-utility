#[cfg(test)]
mod tests {
    use cl_optimizer::{parse_rules, read_file_as_list, topo_sort, Rules};

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

        return true;
    }

    #[test]
    fn test_read_mods() {
        let mods_path = "./tests/modlist.txt";
        assert_eq!(read_file_as_list(&mods_path), vec!["a", "b", "c", "d", "e"])
    }

    #[test]
    fn test_parse_rules() {
        let rules_path = "./tests/rules_base.txt";
        assert!(parse_rules(&rules_path).is_ok(), "rules parsing failed")
    }
}
