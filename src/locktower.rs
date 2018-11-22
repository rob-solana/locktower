use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Clone, Default)]
pub struct Branch {
    id: usize,
    base: usize,
}

impl Branch {
    fn is_derived(&self, other: &Branch, branch_tree: &HashMap<usize, Branch>) -> bool {
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

#[derive(Clone, Default)]
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
    pub fn is_derived(&self, other: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.branch.is_derived(&other.branch, branch_tree)
    }
}

struct VoteLocks {
    votes: VecDeque<Vote>,
    max_size: usize,
    max_height: usize,
}

impl VoteLocks {
    fn new(max_size: usize, max_height: usize) -> Self {
        VoteLocks {
            votes: VecDeque::new(),
            max_size,
            max_height,
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
    fn push_vote(&mut self, vote: Vote) {
        assert!(vote.height <= vote.height);
        assert!(!self.is_full());
        // double the previous lockouts
        for v in &mut self.votes {
            v.lockout *= 2;
        }
        // push the new vote to the font
        self.votes.push_front(vote);
    }
    fn pop_full(&mut self) {
        assert!(self.is_full());
        let _ = self.votes.pop_back();
    }
    fn is_full(&self) -> bool {
        self.votes.len() == self.max_size
    }
    fn is_empty(&self) -> bool {
        self.votes.is_empty()
    }
    fn last_vote(&self) -> Option<&Vote> {
        self.votes.front()
    }
    fn is_vote_valid(&self, vote: &Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.last_vote()
            .map(|v| v.is_derived(vote, branch_tree))
            .unwrap_or(true)
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
        vote_locks.push(VoteLocks::new(MAX_VOTES, 1 << MAX_VOTES));
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
    fn collapse(&mut self) {
        loop {
            if self.vote_locks.len() == 1 {
                break;
            }
            if !self.last_q().is_full() {
                break;
            }
            let full = self.vote_locks.pop().unwrap();
            for v in full.votes.into_iter() {
                self.last_q_mut().push_vote(v);
            }
        }
    }
    fn last_q_mut(&mut self) -> &mut VoteLocks {
        self.vote_locks.last_mut().unwrap()
    }
    fn last_q(&self) -> &VoteLocks {
        self.vote_locks.last().unwrap()
    }
    pub fn push_vote(&mut self, vote: Vote, branch_tree: &HashMap<usize, Branch>) -> bool {
        self.rollback(vote.height);
        let num_old = self.last_q_mut().rollback(vote.height);
        if num_old > 0 && !self.last_q().is_empty() {
            let max_height = self.last_q().last_vote().unwrap().lock_height();
            self.vote_locks.push(VoteLocks::new(num_old, max_height));
        }
        if !self.last_q().is_vote_valid(&vote, branch_tree) {
            return false ;
        }
        self.last_q_mut().push_vote(vote);
        self.collapse();
        if self.last_q().is_full() {
            self.last_q_mut().pop_full();
        }
        true
    }

    pub fn last_branch(&mut self) -> Branch {
        self.last_q()
            .last_vote()
            .map(|v| v.branch.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cmp;

    fn create_network(sz: usize) -> Vec<LockTower> {
        (0..sz).into_iter().map(|_| LockTower::default()).collect()
    }
    /// find the max number of nodes that are on the same branch
    fn converged(network: &Vec<LockTower>, branch_tree: &HashMap<usize, Branch>) -> usize {
        let mut max_derived = 0;
        let mut branches: HashMap<usize, bool> = HashMap::new();
        for n in network {
            if n.last_q().last_vote().is_none() {
                continue;
            }
            let branch = &n.last_q().last_vote().unwrap().branch;
            if branches.get(&branch.id).is_some() {
                continue;
            }
            let derived = network
                .iter()
                .filter(|x| !x.last_q().last_vote().is_none())
                .filter(|y| branch.is_derived(&y.last_q().last_vote().unwrap().branch, branch_tree))
                .count();
            branches.insert(branch.id, true);
            max_derived = cmp::max(max_derived, derived);
        }
        max_derived
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
                for node in network.iter_mut() {
                    assert!(node.push_vote(vote.clone(), &tree));
                }
            }
        }
        assert_eq!(converged(&network, &tree), len);
    }
    #[test]
    fn test_all_partitions() {
        let mut tree = HashMap::new();
        let len = 100;
        let mut network = create_network(len);
        let warmup = 12;
        for rounds in 0..warmup {
            for (i, node) in network.iter_mut().enumerate() {
                let height = rounds * len + i;
                let branch = Branch { id: i + 1, base: 0 };
                tree.insert(branch.id, branch.clone());
                let vote = Vote::new(branch, height);
                assert!(node.push_vote(vote.clone(), &tree));
            }
        }
        for node in network.iter() {
            assert_eq!(node.vote_locks.len(), warmup);
        }
        assert_eq!(converged(&network, &tree), 1);
        for rounds in warmup..warmup + 10 {
            for i in 0..network.len() {
                let height = rounds * len + i;
                let branch = network[i].last_q().last_vote().unwrap().branch.clone();
                let vote = Vote::new(branch, height);
                for node in network.iter_mut() {
                    node.push_vote(vote.clone(), &tree);
                }
                println!("{} {}", height, converged(&network, &tree));
            }
        }
        assert_eq!(converged(&network, &tree), 100);
    } 
}
