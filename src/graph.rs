//! Commit graph lane assignment: a pure function of (sha, parents) in topological order.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    Upper,
    Lower,
}

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub half: Half,
    pub color: usize,
}

#[derive(Clone, Debug, Default)]
pub struct GraphRow {
    pub lane: usize,
    pub color: usize,
    pub edges: Vec<Edge>,
}

pub fn compute(commits: &[(String, Vec<String>)]) -> (Vec<GraphRow>, usize) {
    let mut lanes: Vec<Option<String>> = vec![];
    let mut rows = Vec::with_capacity(commits.len());
    let mut max_lanes = 1;
    for (sha, parents) in commits {
        let before = lanes.clone();
        let node = match before.iter().position(|l| l.as_deref() == Some(sha.as_str())) {
            Some(i) => i,
            None => lanes.iter().position(|l| l.is_none()).unwrap_or_else(|| {
                lanes.push(None);
                lanes.len() - 1
            }),
        };
        let mut edges: Vec<Edge> = vec![];
        for (i, l) in before.iter().enumerate() {
            if let Some(s) = l {
                if s == sha {
                    edges.push(Edge { from: i, to: node, half: Half::Upper, color: i });
                } else {
                    edges.push(Edge { from: i, to: i, half: Half::Upper, color: i });
                    edges.push(Edge { from: i, to: i, half: Half::Lower, color: i });
                }
            }
        }
        // close every lane that was waiting for this commit
        for l in lanes.iter_mut() {
            if l.as_deref() == Some(sha.as_str()) {
                *l = None;
            }
        }
        for (k, par) in parents.iter().enumerate() {
            if let Some(j) = lanes.iter().position(|l| l.as_deref() == Some(par.as_str())) {
                edges.push(Edge { from: node, to: j, half: Half::Lower, color: if k == 0 { node } else { j } });
            } else if k == 0 {
                if lanes.len() <= node {
                    lanes.resize(node + 1, None);
                }
                lanes[node] = Some(par.clone());
                edges.push(Edge { from: node, to: node, half: Half::Lower, color: node });
            } else {
                let j = lanes.iter().position(|l| l.is_none()).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                });
                lanes[j] = Some(par.clone());
                edges.push(Edge { from: node, to: j, half: Half::Lower, color: j });
            }
        }
        while matches!(lanes.last(), Some(None)) {
            lanes.pop();
        }
        max_lanes = max_lanes.max(lanes.len()).max(node + 1);
        rows.push(GraphRow { lane: node, color: node, edges });
    }
    (rows, max_lanes)
}
