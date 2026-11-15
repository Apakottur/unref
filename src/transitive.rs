//! The nodes of a graph that are reached only from outside their strongly connected component.

/// Find the nodes of a directed graph that are reached from outside their strongly connected
/// component.
///
/// The nodes that only reach each other, such as two functions that call each other, form a
/// component. A component that no other component reaches is a root, and the others are transitive:
/// removing the roots, and then what they alone reached, in turn, removes them.
///
/// `successors[node]` holds the nodes that each node reaches, by their index.
pub fn transitive(successors: &[Vec<usize>]) -> Vec<bool> {
    let component = components(successors);
    let mut reached = vec![false; successors.len()];
    for (node, nodes) in successors.iter().enumerate() {
        for &successor in nodes {
            if component[successor] != component[node] {
                reached[component[successor]] = true;
            }
        }
    }
    (0..successors.len())
        .map(|node| reached[component[node]])
        .collect()
}

/// Find the strongly connected components of a directed graph, with Tarjan's algorithm.
///
/// The walk keeps its own stack rather than recursing, so that a long chain does not overflow the
/// stack.
///
/// Returns the component of each node, by one of its nodes.
fn components(successors: &[Vec<usize>]) -> Vec<usize> {
    const UNVISITED: usize = usize::MAX;
    let count = successors.len();
    let mut index = vec![UNVISITED; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack: Vec<usize> = Vec::new();
    let mut component = vec![0; count];
    let mut next_index = 0;

    for start in 0..count {
        if index[start] != UNVISITED {
            continue;
        }
        // Each node being walked, with the position of its next successor.
        let mut walk: Vec<(usize, usize)> = Vec::new();
        index[start] = next_index;
        low[start] = next_index;
        next_index += 1;
        stack.push(start);
        on_stack[start] = true;
        walk.push((start, 0));

        while let Some(&mut (node, ref mut position)) = walk.last_mut() {
            if let Some(&successor) = successors[node].get(*position) {
                *position += 1;
                if index[successor] == UNVISITED {
                    index[successor] = next_index;
                    low[successor] = next_index;
                    next_index += 1;
                    stack.push(successor);
                    on_stack[successor] = true;
                    walk.push((successor, 0));
                } else if on_stack[successor] {
                    low[node] = low[node].min(index[successor]);
                }
                continue;
            }
            walk.pop();
            if let Some(&(parent, _)) = walk.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == index[node] {
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component[member] = node;
                    if member == node {
                        break;
                    }
                }
            }
        }
    }
    component
}

#[cfg(test)]
mod tests {
    use super::transitive;

    #[test]
    fn chain() {
        // 0 -> 1 -> 2: only the root is not transitive.
        assert_eq!(transitive(&[vec![1], vec![2], vec![]]), [false, true, true]);
    }

    #[test]
    fn cycle() {
        // 0 <-> 1 is a root component, and 2 is reached from it.
        assert_eq!(
            transitive(&[vec![1], vec![0, 2], vec![]]),
            [false, false, true]
        );
    }

    #[test]
    fn long_chain() {
        let count = 100_000;
        let successors: Vec<Vec<usize>> = (0..count)
            .map(|node| {
                if node + 1 < count {
                    vec![node + 1]
                } else {
                    vec![]
                }
            })
            .collect();
        let found = transitive(&successors);
        assert!(!found[0]);
        assert!(found[1..].iter().all(|&reached| reached));
    }
}
