#[cfg(test)]
mod topo_tests {
    use petgraph::visit::{Bfs, DfsPostOrder, Topo};
    use petgraph::Graph;
    use std::collections::HashMap;
    use topological_sort::TopologicalSort;
    use toposort_scc::IndexGraph;

    #[test]
    fn test_topological_sort() {
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
    fn test_toposort_scc() {
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
        let result_idx = sort.unwrap();
        println!("{result_idx:?}");

        // check
        let result: Vec<&str> = result_idx.iter().map(|e| mods[*e]).collect();
        println!("{result:?}");
        assert!(checkresult(&result, &rules), "order is wrong")
    }

    #[test]
    fn test_petgraph_topo() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];
        let mut dict: HashMap<&str, u32> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            dict.insert(*m, u32::try_from(i).unwrap());
        }

        let mut g = Graph::<&str, &str>::new();

        // add edges
        for m in &mods {
            g.add_node(*m);
        }
        let mut edges: Vec<(u32, u32)> = vec![];
        for (a, b) in &rules {
            if mods.contains(a) && mods.contains(b) {
                edges.push((dict[a], dict[b]));
            }
        }
        g.extend_with_edges(&edges);

        // print
        let mut topo = Topo::new(&g);
        let mut result_idx: Vec<usize> = Vec::new();
        while let Some(s) = topo.next(&g) {
            result_idx.push(s.index());
        }
        println!("{result_idx:?}");
        let result: Vec<&str> = result_idx.iter().map(|e| mods[*e]).collect();
        println!("{result:?}");

        // check
        assert!(checkresult(&result, &rules), "order is wrong")
    }

    #[test]
    fn test_petgraph_bfs() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];
        let mut dict: HashMap<&str, u32> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            dict.insert(*m, u32::try_from(i).unwrap());
        }

        let mut g = Graph::<&str, &str>::new();

        // add edges
        for m in &mods {
            g.add_node(*m);
        }
        let mut edges: Vec<(u32, u32)> = vec![];
        for (a, b) in &rules {
            if mods.contains(a) && mods.contains(b) {
                edges.push((dict[a], dict[b]));
            }
        }
        g.extend_with_edges(&edges);

        // print
        print_bfs(&g, &mods, 0);
        print_bfs(&g, &mods, 1);
        print_bfs(&g, &mods, 2);
        print_bfs(&g, &mods, 3);
        print_bfs(&g, &mods, 4);
        print_bfs(&g, &mods, 5);
    }

    fn print_bfs(g: &Graph<&str, &str>, mods: &[&str], start: u32) {
        let result_idx = bfs(g, start);
        println!("[{start}]: {result_idx:?}");
        let result: Vec<&str> = result_idx.iter().map(|e| mods[*e]).collect();
        let start_node = mods[start as usize];
        println!("[{start_node}]: {result:?}");
    }

    fn bfs(g: &Graph<&str, &str>, start: u32) -> Vec<usize> {
        let mut topo = Bfs::new(&g, start.into());
        let mut result_idx: Vec<usize> = Vec::new();
        while let Some(s) = topo.next(&g) {
            result_idx.push(s.index());
        }
        result_idx
    }

    #[test]
    fn test_petgraph_dfspo() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];
        let mut dict: HashMap<&str, u32> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            dict.insert(*m, u32::try_from(i).unwrap());
        }

        let mut g = Graph::<&str, &str>::new();

        // add edges
        for m in &mods {
            g.add_node(*m);
        }
        let mut edges: Vec<(u32, u32)> = vec![];
        for (a, b) in &rules {
            if mods.contains(a) && mods.contains(b) {
                edges.push((dict[a], dict[b]));
            }
        }
        g.extend_with_edges(&edges);

        // print
        print_dfspo(&g, &mods, 0);
        print_dfspo(&g, &mods, 1);
        print_dfspo(&g, &mods, 2);
        print_dfspo(&g, &mods, 3);
        print_dfspo(&g, &mods, 4);
        print_dfspo(&g, &mods, 5);
    }

    fn print_dfspo(g: &Graph<&str, &str>, mods: &[&str], start: u32) {
        let result_idx = dfspo(g, start);
        println!("[{start}]: {result_idx:?}");
        let result: Vec<&str> = result_idx.iter().map(|e| mods[*e]).collect();
        let start_node = mods[start as usize];
        println!("[{start_node}]: {result:?}");
    }

    fn dfspo(g: &Graph<&str, &str>, start: u32) -> Vec<usize> {
        let mut topo = DfsPostOrder::new(&g, start.into());
        let mut result_idx: Vec<usize> = Vec::new();
        while let Some(s) = topo.next(&g) {
            result_idx.push(s.index());
        }
        result_idx
    }

    #[test]
    fn test_petgraph_sort() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];
        let mut index_dict: HashMap<&str, u32> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            index_dict.insert(*m, u32::try_from(i).unwrap());
        }
        let mut edges: Vec<(u32, u32)> = vec![];
        for (a, b) in &rules {
            if mods.contains(a) && mods.contains(b) {
                edges.push((index_dict[a], index_dict[b]));
                println!("Edge added from {} to {}", index_dict[a], index_dict[b]);
            }
        }

        // construct rule graph
        let mut r = Graph::<&str, &str>::new();
        for m in &mods {
            r.add_node(*m);
        }
        r.extend_with_edges(&edges);
        // todo check if has cycle
        //

        // construct main graph
        let mut g = r.clone();

        // add edge from a to b only if no path from b to a in the rule graph (check this!!)
        for i in 0..mods.len() - 1 {
            let a = index_dict[mods[i]];
            let b = index_dict[mods[i + 1]];

            // check dfspo
            //let mut dfspo = DfsPostOrder::new(&r, a.into());
            let has_path = dfspo(&r, b).contains(&(a as usize));
            if !has_path {
                if !g.contains_edge(a.into(), b.into()) {
                    g.add_edge(a.into(), b.into(), "");
                    println!("Edge added from {} to {}", a, b);
                }
            } else {
                println!("Edge skipped from {} to {}", a, b);
            }
        }

        // topo sort the main graph
        let mut topo = Topo::new(&g);
        let mut result_idx: Vec<usize> = Vec::new();
        while let Some(s) = topo.next(&g) {
            result_idx.push(s.index());
        }

        // print
        println!("{result_idx:?}");
        let result: Vec<&str> = result_idx.iter().map(|e| mods[*e]).collect();
        println!("{result:?}");

        assert!(checkresult(&result, &rules), "order is wrong")
    }

    #[test]
    fn test_petgraph_stable_sort() {
        let mods = vec!["a", "b", "c", "d", "e", "f"];
        let rules = vec![("a", "b"), ("b", "c"), ("d", "e"), ("e", "c")];
        let mut index_dict: HashMap<&str, u32> = HashMap::new();
        for (i, m) in mods.iter().enumerate() {
            index_dict.insert(*m, u32::try_from(i).unwrap());
        }
        let mut edges: Vec<(u32, u32)> = vec![];
        for (a, b) in &rules {
            if mods.contains(a) && mods.contains(b) {
                edges.push((index_dict[a], index_dict[b]));
                println!("Edge added from {} to {}", index_dict[a], index_dict[b]);
            }
        }

        // construct rule graph
        let mut r = Graph::<&str, &str>::new();
        for m in &mods {
            r.add_node(*m);
        }
        r.extend_with_edges(&edges);
        // todo check if has cycle
        //

        // construct main graph
        //let mut g = r.clone();
        let mut result: Vec<String> = mods.clone().iter().map(|e| (*e).to_owned()).collect();
        println!("{result:?}");
        loop {
            if !petgraph_stable_sort(mods.len(), &r, &index_dict, &mut result) {
                break;
            }
        }

        // print
        println!("{result:?}");

        //assert!(checkresult(&result, &rules))
    }

    fn petgraph_stable_sort(
        n: usize,
        r: &Graph<&str, &str>,
        index_dict: &HashMap<&str, u32>,
        result: &mut Vec<String>,
    ) -> bool {
        for i in 0..n {
            for j in 0..i {
                let x = index_dict[result[i].as_str()];
                let y = index_dict[result[j].as_str()];
                if r.contains_edge(x.into(), y.into()) {
                    //println!("{result:?}");
                    let t = result[i].clone();
                    result.remove(i);
                    result.insert(j, t);
                    println!("[{x}-{y}] {result:?}");
                    return true;
                }
            }
        }
        false
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
