#[cfg(test)]
mod unit_tests {
    use cmop::{topo_sort, Rules};

    #[test]
    fn test_cycle() {
        let rules = Rules {
            order: vec![("a", "b"), ("b", "c"), ("d", "e"), ("b", "a")]
                .iter()
                .map(|e| (e.0.to_owned(), e.1.to_owned()))
                .collect(),
        };

        let mods: Vec<String> = vec!["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        assert!(
            topo_sort(&mods, &rules).is_err(),
            "rules do not contain a cycle"
        )
    }

    #[test]
    fn test_ordering() {
        let rules = Rules {
            order: vec![
                ("a", "b"),
                ("b", "c"),
                ("d", "e"),
                ("e", "c"),
                ("test.archive", "test2.archive"),
            ]
            .iter()
            .map(|e| (e.0.to_owned(), e.1.to_owned()))
            .collect(),
        };

        let mods = vec!["a", "b", "c", "d", "e", "f", "g"]
            .iter()
            .map(|e| (*e).into())
            .collect();

        match topo_sort(&mods, &rules) {
            Ok(result) => assert!(checkresult(&result, &rules), "order is wrong"),
            Err(_) => panic!("rules contain a cycle"),
        }
    }

    fn checkresult(result: &[String], rules: &Rules) -> bool {
        let pairs = &rules.order;
        for (a, b) in pairs {
            let pos_a = result.iter().position(|x| x == a);
            if pos_a.is_none() {
                continue;
            }
            let pos_b = result.iter().position(|x| x == b);
            if pos_b.is_none() {
                continue;
            }

            if pos_a.unwrap() > pos_b.unwrap() {
                return false;
            }
        }

        true
    }
}
