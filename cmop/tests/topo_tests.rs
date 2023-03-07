#[cfg(test)]
mod topo_tests {
    use petgraph::visit::Topo;
    use petgraph::Graph;
    use std::collections::HashMap;
    use topological_sort::TopologicalSort;
    use toposort_scc::IndexGraph;

    #[test]
    fn test_topo_sort1() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];

        let mut ts = TopologicalSort::<&str>::new();
        // add edges
        for m in &mods {
            ts.insert(*m);
        }
        for pair in &rules {
            if mods.contains(&pair.0) && mods.contains(&pair.1) {
                ts.add_dependency(pair.0, pair.1);
            }
        }
        // sort
        let mut result: Vec<&str> = Vec::new();
        while let Some(s) = ts.pop() {
            result.push(s);
        }
        let len = ts.len();
        assert_eq!(len, 0, "Graph contains a cycle");
        println!("{result:?}");

        // check
        assert!(checkresult(&result, &rules), "order is wrong")
    }

    #[test]
    fn test_topo_sort2() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];

        let mut g = IndexGraph::with_vertices(mods.len());
        let mut dict: HashMap<&str, usize> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            dict.insert(*m, i);
        }
        // add edges
        for (a, b) in &rules {
            if mods.contains(a) && mods.contains(b) {
                let idx_a = dict[a];
                let idx_b = dict[b];
                g.add_edge(idx_a, idx_b);
            }
        }
        // sort
        let sort = g.toposort();
        assert!(sort.is_some(), "Graph contains a cycle");
        let idx = sort.unwrap();
        println!("{idx:?}");
        let result: Vec<&str> = idx.iter().map(|e| mods[*e]).collect();
        println!("{result:?}");

        // check
        assert!(checkresult(&result, &rules), "order is wrong")
    }

    #[test]
    fn test_topo_sort3() {
        let mut g = Graph::<i32, (), petgraph::Directed, usize>::from_edges(&[
            (0, 1),
            (1, 2),
            (3, 4),
            //(4, 2),
        ]);
        g.add_node(5);

        let mut topo = Topo::new(&g);
        let mut result: Vec<usize> = Vec::new();
        while let Some(s) = topo.next(&g) {
            result.push(s.index());
        }
        println!("{result:?}");
    }

    fn checkresult(result: &[&str], rules: &Vec<(&str, &str)>) -> bool {
        for (a, b) in rules {
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
