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
    height: usize,
    lockout: usize,
}

impl Vote {
    pub fn new(branch: Branch, height: usize) -> Vote {
        Self {
            branch,
            height,
            lockout: 2,
        }
    }
    pub fn lock_height(&self) -> usize {
        self.height + self.lockout
    }
    pub fn is_trunk_of(&self, other: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.branch.is_trunk_of(&other.branch, branch_tree)
    }
}

#[derive(Debug)]
pub struct LockTower {
    votes: VecDeque<Vote>,
    max_size: usize,
}

impl LockTower {
    pub fn new(max_size: usize) -> Self {
        Self {
            votes: VecDeque::new(),
            max_size,
        }
    }
    pub fn push_vote(
        &mut self,
        vote: Vote,
        branch_tree: &HashMap<usize, Branch>,
        converge_map: &HashMap<usize, usize>,
        depth: usize,
    ) -> bool {
        self.rollback(vote.height);
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
    fn is_converged(&self, converge_map: &HashMap<usize, usize>, depth: usize) -> bool {
        self.get_vote(depth)
            .map(|v| *converge_map.get(&v.branch.id).unwrap_or(&0) > 50)
            .unwrap_or(true)
    }

    fn rollback(&mut self, height: usize) {
        let mut last: isize = -1;
        for (i, v) in self.votes.iter().enumerate() {
            if v.lock_height() < height {
                last = i as isize;
            }
        }
        for _ in 0..(last + 1) {
            self.votes.pop_front();
        }
    }
    fn is_valid(&mut self, vote: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        for v in &self.votes {
            if !v.is_trunk_of(&vote, branch_tree) {
                return false;
            }
        }
        true
    }
    fn enter_vote(&mut self, vote: Vote) {
        assert!(!self.is_full());
        // double the previous lockouts
        for (i, v) in self.votes.iter_mut().enumerate() {
            assert!(v.height <= vote.height);
            if (v.lockout + 1) < 1 << (i + 2) {
                v.lockout *= 2;
            }
        }
        assert_eq!(vote.lockout, 2);
        // push the new vote to the font
        self.votes.push_front(vote);
    }
    fn pop_full(&mut self) {
        assert!(self.is_full());
        let _ = self.votes.pop_back();
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
            .unwrap_or(Branch::default())
    }
}

pub const MAX_VOTES: usize = 32usize;
pub const FINALITY_DEPTH: usize = 8;

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
        let cmap = HashMap::new();
        let b0 = Branch { id: 0, base: 0 };
        let mut node = LockTower::new(32);
        let vote = Vote::new(b0.clone(), 0);
        assert!(node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.votes.len(), 1);

        let vote = Vote::new(b0.clone(), 1);
        assert!(node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.votes.len(), 2);

        let vote = Vote::new(b0.clone(), 2);
        assert!(node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.votes.len(), 3);

        let vote = Vote::new(b0.clone(), 3);
        assert!(node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.votes.len(), 4);

        assert_eq!(node.votes[0].lockout, 2);
        assert_eq!(node.votes[1].lockout, 4);
        assert_eq!(node.votes[2].lockout, 8);
        assert_eq!(node.votes[3].lockout, 16);

        assert_eq!(node.votes[1].lock_height(), 6);
        assert_eq!(node.votes[2].lock_height(), 9);

        let vote = Vote::new(b0.clone(), 7);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        assert_eq!(node.votes[0].lockout, 2);

        let b1 = Branch { id: 1, base: 1 };
        let vote = Vote::new(b1.clone(), 8);
        assert!(!node.push_vote(vote, &tree, &cmap, 32));

        let vote = Vote::new(b0.clone(), 8);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        assert_eq!(node.votes.len(), 4);
        assert_eq!(node.votes[0].lockout, 2);
        assert_eq!(node.votes[1].lockout, 4);
        assert_eq!(node.votes[2].lockout, 8);
        assert_eq!(node.votes[3].lockout, 16);

        let vote = Vote::new(b0.clone(), 10);
        assert!(node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.votes.len(), 2);
    }

    fn create_network(sz: usize) -> Vec<LockTower> {
        (0..sz).into_iter().map(|_| LockTower::new(32)).collect()
    }
    fn calc_converge_map(
        network: &Vec<LockTower>,
        branch_tree: &HashMap<usize, Branch>,
    ) -> HashMap<usize, usize> {
        let mut cmap: HashMap<usize, usize> = HashMap::new();
        for node in network {
            if cmap.get(&node.last_branch().id).is_some() {
                continue;
            }
            let common = network
                .iter()
                .filter(|y| {
                    node.last_branch()
                        .is_trunk_of(&y.last_branch(), branch_tree)
                }).count();
            cmap.insert(node.last_branch().id, common);
        }
        cmap
    }
    fn calc_converged(cmap: &HashMap<usize, usize>) -> usize {
        let len: usize = cmap.values().len();
        let sum: usize = cmap.values().sum();
        sum / len
    }
    #[test]
    fn test_no_partitions() {
        let tree = HashMap::new();
        let len = 100;
        let mut network = create_network(len);
        for rounds in 0..100 {
            for i in 0..network.len() {
                let height = rounds * len + i;
                let branch = Branch { id: 0, base: 0 };
                let vote = Vote::new(branch, height);
                let cmap = calc_converge_map(&network, &tree);
                for node in network.iter_mut() {
                    assert!(node.push_vote(vote.clone(), &tree, &cmap, 0));
                }
            }
        }
        let cmap = calc_converge_map(&network, &tree);
        assert_eq!(calc_converged(&cmap), len);
    }
    fn test_with_partitions(num_partitions: usize) {
        let mut tree = HashMap::new();
        let len = 100;
        let mut network = create_network(len);
        let fail_rate = 0.5;
        let warmup = 7;
        for height in 0..warmup {
            let cmap = calc_converge_map(&network, &tree);
            for node in network.iter_mut() {
                let mut branch = node.last_branch().clone();
                if branch.id == 0 {
                    branch.id = thread_rng().gen_range(1, 1 + num_partitions);
                    tree.insert(branch.id, branch.clone());
                }
                let vote = Vote::new(branch, height);
                assert!(node.is_valid(&vote, &tree));
                assert!(node.push_vote(vote.clone(), &tree, &cmap, warmup));
            }
        }
        for node in network.iter() {
            assert_eq!(node.votes.len(), warmup);
            assert_eq!(node.first_vote().unwrap().lockout, 1 << warmup);
            assert!(node.first_vote().unwrap().lock_height() >= 1 << warmup);
        }
        let cmap = calc_converge_map(&network, &tree);
        assert_ne!(calc_converged(&cmap), len);
        for rounds in 0..40 {
            for i in 0..len {
                let height = warmup + rounds * len + i;
                let branch = network[i].last_branch().clone();
                let cmap = calc_converge_map(&network, &tree);
                let vote = Vote::new(branch, height);
                for node in network.iter_mut() {
                    if thread_rng().gen_range(0f64, 1.0f64) < fail_rate {
                        continue;
                    }
                    node.push_vote(vote.clone(), &tree, &cmap, warmup);
                }
                let cmap = calc_converge_map(&network, &tree);
                println!("{} {}", height, calc_converged(&cmap));
            }
            let cmap = calc_converge_map(&network, &tree);
            if calc_converged(&cmap) == len {
                break;
            }
        }
        let cmap = calc_converge_map(&network, &tree);
        assert_eq!(calc_converged(&cmap), len);
    }
    #[test]
    fn test_all_partitions() {
        test_with_partitions(100)
    }
    #[test]
    fn test_2_partitions() {
        test_with_partitions(2)
    }
    #[test]
    fn test_3_partitions() {
        test_with_partitions(3)
    }
}
