use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Clone, Default, Debug)]
pub struct Branch {
    id: usize,
    base: usize,
}

impl Branch {
    fn is_trunk_of(&self, other: &Branch, branch_tree: &HashMap<usize, Branch>) -> bool {
        let mut current = other.clone();
        loop {
            // found it
            if current.id == self.id {
                return true;
            }
            // base is 0, and this id is 0
            if current.base == 0 && self.id == 0 {
                assert!(branch_tree.get(&0).is_none());
                return true;
            }
            // base is 0
            if branch_tree.get(&current.base).is_none() {
                return false;
            }
            current = branch_tree.get(&current.base).unwrap().clone();
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Vote {
    branch: Branch,
    time: usize,
    lockout: usize,
}

impl Vote {
    pub fn new(branch: Branch, time: usize) -> Vote {
        Self {
            branch,
            time,
            lockout: 2,
        }
    }
    pub fn lock_height(&self) -> usize {
        self.time + self.lockout
    }
    pub fn is_trunk_of(&self, other: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.branch.is_trunk_of(&other.branch, branch_tree)
    }
}

#[derive(Debug)]
pub struct LockTower {
    votes: VecDeque<Vote>,
    max_size: usize,
    branch_trunk: Branch,
}

impl LockTower {
    pub fn new(max_size: usize) -> Self {
        Self {
            votes: VecDeque::new(),
            max_size,
            branch_trunk: Branch::default(),
        }
    }
    pub fn push_vote(
        &mut self,
        vote: Vote,
        branch_tree: &HashMap<usize, Branch>,
        converge_map: &HashMap<usize, usize>,
        depth: usize,
    ) -> bool {
        self.rollback(vote.time);
        if !self.is_valid(&vote, branch_tree) {
            return false;
        }
        if !self.is_converged(converge_map, depth) {
            return false;
        }
        self.enter_vote(vote);
        if self.is_full() {
            self.pop_full();
        }
        true
    }
    /// check if the vote at `depth` has over 50% of the network committed
    fn is_converged(&self, converge_map: &HashMap<usize, usize>, depth: usize) -> bool {
        self.get_vote(depth)
            .map(|v| {
                let v = *converge_map.get(&v.branch.id).unwrap_or(&0);
                // hard coded to 100 nodes
                assert!(v <= 100);
                v > 50
            }).unwrap_or(true)
    }

    /// if a vote is expired, pop it and all the votes leading up to it
    fn rollback(&mut self, time: usize) {
        let mut last: isize = -1;
        for (i, v) in self.votes.iter().enumerate() {
            if v.lock_height() < time {
                last = i as isize;
            }
        }
        for _ in 0..(last + 1) {
            self.votes.pop_front();
        }
    }
    /// only add votes that are descendent from the last vote in the stack
    fn is_valid(&mut self, vote: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.last_branch().is_trunk_of(&vote.branch, branch_tree)
    }
    fn enter_vote(&mut self, vote: Vote) {
        let vote_time = vote.time;
        assert!(!self.is_full());
        assert_eq!(vote.lockout, 2);
        // push the new vote to the font
        self.votes.push_front(vote);
        // double the lockouts if the threshold to doulbe is met
        for i in 1..self.votes.len() {
            assert!(self.votes[i].time <= vote_time);
            if self.votes[i].lockout == self.votes[i - 1].lockout {
                self.votes[i].lockout *= 2;
            }
        }
    }
    fn pop_full(&mut self) {
        assert!(self.is_full());
        self.branch_trunk = self.votes.pop_back().unwrap().branch;
    }
    fn is_full(&self) -> bool {
        assert!(self.votes.len() <= self.max_size);
        self.votes.len() == self.max_size
    }
    fn last_vote(&self) -> Option<&Vote> {
        self.votes.front()
    }
    fn get_vote(&self, ix: usize) -> Option<&Vote> {
        self.votes.get(ix)
    }
    pub fn first_vote(&self) -> Option<&Vote> {
        self.votes.back()
    }
    pub fn last_branch(&self) -> Branch {
        self.last_vote()
            .map(|v| v.branch.clone())
            .unwrap_or(self.branch_trunk.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::{thread_rng, Rng};

    #[test]
    fn test_is_trunk_of_1() {
        let tree = HashMap::new();
        let b1 = Branch { id: 1, base: 0 };
        let b2 = Branch { id: 2, base: 0 };
        assert!(!b1.is_trunk_of(&b2, &tree));
    }
    #[test]
    fn test_is_trunk_of_2() {
        let tree = HashMap::new();
        let b1 = Branch { id: 1, base: 0 };
        let b2 = Branch { id: 0, base: 0 };
        assert!(!b1.is_trunk_of(&b2, &tree));
    }
    #[test]
    fn test_is_trunk_of_3() {
        let tree = HashMap::new();
        let b1 = Branch { id: 1, base: 0 };
        let b2 = Branch { id: 1, base: 0 };
        assert!(b1.is_trunk_of(&b2, &tree));
    }
    #[test]
    fn test_is_trunk_of_4() {
        let mut tree = HashMap::new();
        let b1 = Branch { id: 1, base: 0 };
        let b2 = Branch { id: 2, base: 1 };
        tree.insert(b1.id, b1.clone());
        assert!(b1.is_trunk_of(&b2, &tree));
        assert!(!b2.is_trunk_of(&b1, &tree));
    }
    #[test]
    fn test_push_vote() {
        let tree = HashMap::new();
        let bmap = HashMap::new();
        let b0 = Branch { id: 0, base: 0 };
        let mut node = LockTower::new(32);
        let vote = Vote::new(b0.clone(), 0);
        assert!(node.push_vote(vote, &tree, &bmap, 32));
        assert_eq!(node.votes.len(), 1);

        let vote = Vote::new(b0.clone(), 1);
        assert!(node.push_vote(vote, &tree, &bmap, 32));
        assert_eq!(node.votes.len(), 2);

        let vote = Vote::new(b0.clone(), 2);
        assert!(node.push_vote(vote, &tree, &bmap, 32));
        assert_eq!(node.votes.len(), 3);

        let vote = Vote::new(b0.clone(), 3);
        assert!(node.push_vote(vote, &tree, &bmap, 32));
        assert_eq!(node.votes.len(), 4);

        assert_eq!(node.votes[0].lockout, 2);
        assert_eq!(node.votes[1].lockout, 4);
        assert_eq!(node.votes[2].lockout, 8);
        assert_eq!(node.votes[3].lockout, 16);

        assert_eq!(node.votes[1].lock_height(), 6);
        assert_eq!(node.votes[2].lock_height(), 9);

        let vote = Vote::new(b0.clone(), 7);
        assert!(node.push_vote(vote, &tree, &bmap, 32));

        assert_eq!(node.votes[0].lockout, 2);

        let b1 = Branch { id: 1, base: 1 };
        let vote = Vote::new(b1.clone(), 8);
        assert!(!node.push_vote(vote, &tree, &bmap, 32));

        let vote = Vote::new(b0.clone(), 8);
        assert!(node.push_vote(vote, &tree, &bmap, 32));

        assert_eq!(node.votes.len(), 4);
        assert_eq!(node.votes[0].lockout, 2);
        assert_eq!(node.votes[1].lockout, 4);
        assert_eq!(node.votes[2].lockout, 8);
        assert_eq!(node.votes[3].lockout, 16);

        let vote = Vote::new(b0.clone(), 10);
        assert!(node.push_vote(vote, &tree, &bmap, 32));
        assert_eq!(node.votes.len(), 2);
        assert_eq!(node.votes[0].lockout, 2);
        assert_eq!(node.votes[1].lockout, 16);
    }

    fn create_network(sz: usize) -> Vec<LockTower> {
        (0..sz).into_iter().map(|_| LockTower::new(32)).collect()
    }

    /// The "height" or "depth" of this branch. How many branches until it connects to branch 0
    fn calc_branch_depth(branch_tree: &HashMap<usize, Branch>, id: usize) -> usize {
        let mut depth = 0;
        let mut start = branch_tree.get(&id);
        loop {
            if start.is_none() {
                break;
            }
            depth += 1;
            start = branch_tree.get(&start.unwrap().base);
        }
        depth
    }
    /// map of `branch id` to `node count`
    /// This map contains how many nodes have the branch as an ancestor
    /// The branch with the highest count that is the newest is the network "trunk"
    fn calc_branch_map(
        network: &Vec<LockTower>,
        branch_tree: &HashMap<usize, Branch>,
    ) -> HashMap<usize, usize> {
        let mut lca_map: HashMap<usize, usize> = HashMap::new();
        for node in network {
            let mut start = node.last_branch();
            loop {
                *lca_map.entry(start.id).or_insert(0) += 1;
                if branch_tree.get(&start.base).is_none() {
                    break;
                }
                start = branch_tree.get(&start.base).unwrap().clone();
            }
        }
        lca_map
    }
    /// find the branch with the highest count of nodes that have it as an ancestor
    /// as well as with the highest possible branch id, which indicates it is the newest
    fn calc_newest_trunk(bmap: &HashMap<usize, usize>) -> (usize, usize) {
        let mut data: Vec<_> = bmap.iter().collect();
        data.sort_by_key(|x| (x.1, x.0));
        data.last().map(|v| (*v.0, *v.1)).unwrap()
    }
    /// how common is the latest branch of all the nodes
    fn calc_tip_converged(network: &Vec<LockTower>, bmap: &HashMap<usize, usize>) -> usize {
        let sum: usize = network
            .iter()
            .map(|n| *bmap.get(&n.last_branch().id).unwrap_or(&0))
            .sum();
        sum / network.len()
    }
    #[test]
    fn test_no_partitions() {
        let mut tree = HashMap::new();
        let len = 100;
        let mut network = create_network(len);
        for rounds in 0..1 {
            for i in 0..network.len() {
                let time = rounds * len + i;
                let base = network[i].last_branch().clone();
                let branch = Branch {
                    id: time + 1,
                    base: base.id,
                };
                tree.insert(branch.id, branch.clone());
                let vote = Vote::new(branch, time);
                let bmap = calc_branch_map(&network, &tree);
                for node in network.iter_mut() {
                    assert!(node.push_vote(vote.clone(), &tree, &bmap, 0));
                }
                println!("{} {}", time, calc_tip_converged(&network, &bmap));
            }
        }
        let bmap = calc_branch_map(&network, &tree);
        assert_eq!(calc_tip_converged(&network, &bmap), len);
    }
    /// * num_partitions - 1 to 100 partitions
    /// * fail_rate - 0 to 1.0 rate of packet receive failure
    fn test_with_partitions(num_partitions: usize, fail_rate: f64) {
        let mut tree = HashMap::new();
        let len = 100;
        let mut network = create_network(len);
        let warmup = 8;
        for time in 0..warmup {
            let bmap = calc_branch_map(&network, &tree);
            for node in network.iter_mut() {
                let mut branch = node.last_branch().clone();
                if branch.id == 0 {
                    branch.id = thread_rng().gen_range(1, 1 + num_partitions);
                    tree.insert(branch.id, branch.clone());
                }
                let vote = Vote::new(branch, time);
                assert!(node.is_valid(&vote, &tree));
                assert!(node.push_vote(vote.clone(), &tree, &bmap, warmup));
            }
        }
        for node in network.iter() {
            assert_eq!(node.votes.len(), warmup);
            assert_eq!(node.first_vote().unwrap().lockout, 1 << warmup);
            assert!(node.first_vote().unwrap().lock_height() >= 1 << warmup);
        }
        let bmap = calc_branch_map(&network, &tree);
        assert_ne!(calc_tip_converged(&network, &bmap), len);
        for rounds in 0..40 {
            for i in 0..len {
                let time = warmup + rounds * len + i;
                let base = network[i].last_branch().clone();
                let branch = Branch {
                    id: time + num_partitions,
                    base: base.id,
                };
                tree.insert(branch.id, branch.clone());
                let bmap = calc_branch_map(&network, &tree);
                let vote = Vote::new(branch, time);
                for node in network.iter_mut() {
                    if thread_rng().gen_range(0f64, 1.0f64) < fail_rate {
                        continue;
                    }
                    node.push_vote(vote.clone(), &tree, &bmap, warmup);
                }
                let bmap = calc_branch_map(&network, &tree);
                let trunk = calc_newest_trunk(&bmap);
                let trunk_time = if trunk.0 > num_partitions {
                    trunk.0 - num_partitions
                } else {
                    trunk.0
                };
                println!(
                    "time: {}, tip converged: {}, trunk id: {}, trunk time: {}, trunk converged {}, trunk depth {}",
                    time,
                    calc_tip_converged(&network, &bmap),
                    trunk.0,
                    trunk_time,
                    trunk.1,
                    calc_branch_depth(&tree, trunk.0)
                );
                if calc_tip_converged(&network, &bmap) == len {
                    break;
                }
            }
            let bmap = calc_branch_map(&network, &tree);
            if calc_tip_converged(&network, &bmap) == len {
                break;
            }
        }
        let bmap = calc_branch_map(&network, &tree);
        let trunk = calc_newest_trunk(&bmap);
        assert_eq!(trunk.1, len);
    }
    #[test]
    #[ignore]
    fn test_all_partitions() {
        test_with_partitions(100, 0.2)
    }
    #[test]
    fn test_2_partitions() {
        test_with_partitions(2, 0.0)
    }
    #[test]
    #[ignore]
    fn test_3_partitions() {
        test_with_partitions(3, 0.9)
    }
    #[test]
    fn test_4_partitions() {
        test_with_partitions(4, 0.0)
    }
}
