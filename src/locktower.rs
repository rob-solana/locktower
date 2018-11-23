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
pub struct VoteLocks {
    votes: VecDeque<Vote>,
    max_size: usize,
    max_height: usize,
    trunk_branch: Branch,
}

impl VoteLocks {
    pub fn new(max_size: usize, max_height: usize, trunk_branch: Branch) -> Self {
        VoteLocks {
            votes: VecDeque::new(),
            max_size,
            max_height,
            trunk_branch,
        }
    }
    fn rollback(&mut self, height: usize) -> usize {
        let num_old = self
            .votes
            .iter()
            .take_while(|v| v.lock_height() < height)
            .count();
        for _ in 0..num_old {
            self.votes.pop_front();
        }
        num_old
    }
    pub fn push_vote(&mut self, vote: Vote) {
        assert!(vote.height <= vote.height);
        assert!(!self.is_full());
        // double the previous lockouts
        for v in &mut self.votes {
            v.lockout *= 2;
        }
        // push the new vote to the font
        self.votes.push_front(vote);
    }
    fn append(&mut self, mut other: Self, branch_tree: &HashMap<usize, Branch>) {
        for _ in 0..other.votes.len() {
            let v = other.votes.pop_back().unwrap();
            assert!(self.last_branch().is_trunk_of(&v.branch, branch_tree));
            self.votes.push_front(v);
        }
    }
    fn pop_full(&mut self) {
        assert!(self.is_full());
        let _ = self.votes.pop_back();
    }
    fn is_full(&self) -> bool {
        assert!(self.votes.len() <= self.max_size);
        self.votes.len() == self.max_size
    }
    fn is_empty(&self) -> bool {
        self.votes.is_empty()
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
    fn last_branch(&self) -> Branch {
        self.last_vote()
            .map(|v| v.branch.clone())
            .unwrap_or(self.trunk_branch.clone())
    }
    pub fn is_vote_valid(&self, vote: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.last_vote()
            .map(|v| v.is_trunk_of(vote, branch_tree))
            .unwrap_or(self.trunk_branch.is_trunk_of(&vote.branch, branch_tree))
    }
}

pub struct LockTower {
    vote_locks: Vec<VoteLocks>,
}

pub const MAX_VOTES: usize = 32usize;
pub const FINALITY_DEPTH: usize = 8;

impl Default for LockTower {
    fn default() -> Self {
        let mut vote_locks = Vec::new();
        vote_locks.push(VoteLocks::new(MAX_VOTES, 1 << MAX_VOTES, Branch::default()));
        Self { vote_locks }
    }
}

impl LockTower {
    fn rollback(&mut self, height: usize) {
        let num_old = self
            .vote_locks
            .iter()
            .rev()
            .take_while(|v| v.max_height < height)
            .count();
        for _ in 0..num_old {
            self.vote_locks.pop();
        }
        assert!(!self.vote_locks.is_empty());
    }
    fn collapse(&mut self, branch_tree: &HashMap<usize, Branch>) {
        loop {
            if self.vote_locks.len() == 1 {
                break;
            }
            if !self.last_q().is_full() {
                break;
            }
            let full = self.vote_locks.pop().unwrap();
            println!("collapse of q {}", full.votes.len());
            self.last_q_mut().append(full, branch_tree);
        }
    }
    fn last_q_mut(&mut self) -> &mut VoteLocks {
        self.vote_locks.last_mut().unwrap()
    }
    fn last_q(&self) -> &VoteLocks {
        self.vote_locks.last().unwrap()
    }
    // if the vote at depth is not common with more then 50% of the network then we should fail
    // this vote until it is common, or enough votes get unrolled
    pub fn is_converged(&self, converge_map: &HashMap<usize, usize>, depth: usize) -> bool {
        self.last_q()
            .get_vote(depth)
            .map(|v| *converge_map.get(&v.branch.id).unwrap_or(&0) > 50)
            .unwrap_or(true)
    }
    pub fn push_vote(
        &mut self,
        vote: Vote,
        branch_tree: &HashMap<usize, Branch>,
        converge_map: &HashMap<usize, usize>,
        depth: usize,
    ) -> bool {
        self.rollback(vote.height);
        let num_old = self.last_q_mut().rollback(vote.height);
        if num_old > 0 && !self.last_q().is_empty() {
            println!("rollback votes: {}", num_old);
            let last_vote = self.last_q().last_vote().unwrap().clone();
            self.vote_locks.push(VoteLocks::new(
                num_old,
                last_vote.lock_height(),
                last_vote.branch,
            ));
        }
        if !self.last_q().is_vote_valid(&vote, branch_tree) {
            return false;
        }
        if !self.is_converged(converge_map, depth) {
            return false;
        }
        self.last_q_mut().push_vote(vote);
        self.collapse(branch_tree);
        if self.last_q().is_full() {
            assert_eq!(self.vote_locks.len(), 1);
            self.last_q_mut().pop_full();
        }
        true
    }

    pub fn last_branch(&self) -> Branch {
        self.last_q().last_branch()
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
        let mut tree = HashMap::new();
        let cmap = HashMap::new();
        let b0 = Branch { id: 0, base: 0 };
        let mut node = LockTower::default();
        let vote = Vote::new(b0.clone(), 0);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        let vote = Vote::new(b0.clone(), 1);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        let vote = Vote::new(b0.clone(), 2);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        let vote = Vote::new(b0.clone(), 3);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        assert_eq!(node.last_q().votes.len(), 4);
        assert_eq!(node.last_q().votes[0].lockout, 2);
        assert_eq!(node.last_q().votes[1].lockout, 4);
        assert_eq!(node.last_q().votes[2].lockout, 8);
        assert_eq!(node.last_q().votes[3].lockout, 16);

        assert_eq!(node.last_q().votes[1].lock_height(), 6);
        assert_eq!(node.last_q().votes[2].lock_height(), 9);

        let vote = Vote::new(b0.clone(), 7);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        assert_eq!(node.vote_locks.len(), 2);
        assert_eq!(node.last_q().votes[0].lockout, 2);

        let b1 = Branch { id: 1, base: 1 };
        let vote = Vote::new(b1.clone(), 8);
        assert!(!node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.vote_locks.len(), 2);

        let vote = Vote::new(b0.clone(), 8);
        assert!(node.push_vote(vote, &tree, &cmap, 32));

        assert_eq!(node.vote_locks.len(), 1);
        assert_eq!(node.last_q().votes[0].lockout, 2);
        assert_eq!(node.last_q().votes[1].lockout, 4);
        assert_eq!(node.last_q().votes[2].lockout, 8);
        assert_eq!(node.last_q().votes[3].lockout, 16);

        let vote = Vote::new(b0.clone(), 10);
        assert!(node.push_vote(vote, &tree, &cmap, 32));
        assert_eq!(node.vote_locks.len(), 2);
    }

    fn create_network(sz: usize) -> Vec<LockTower> {
        (0..sz).into_iter().map(|_| LockTower::default()).collect()
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
        *cmap.values().min().unwrap_or(&0)
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
        let fail_rate = 0.4;
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
                assert!(node.last_q().is_vote_valid(&vote, &tree));
                assert!(node.push_vote(vote.clone(), &tree, &cmap, warmup));
            }
        }
        for node in network.iter() {
            assert_eq!(node.last_q().votes.len(), warmup);
            assert_eq!(node.last_q().first_vote().unwrap().lockout, 1 << warmup);
            assert!(node.last_q().max_height > 1 << warmup);
            assert!(node.last_q().first_vote().unwrap().lock_height() >= 1 << warmup);
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
                    println!("{:?}", node.last_q());
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
