use std::collections::VecDeque;

pub struct Branch {
    branches: Vec<(usize, usize)>,
}

pub impl Branch {
    fn is_derived(&self, other: &Branch) {
        self.branches.iter().zip(&other.branches).all(|(x,y)| x == y)
    }
}

#[derive(Clone, Default)]
pub struct Vote {
    branch: Branch,
    height: usize,
    lockout: usize,
}

impl Vote {
    pub fn new(branch: Branch) -> Vote {
        Vote {branch, lockout: 2}
    }
    pub fn lock_height(&self) -> usize {
        self.height + self.lockout 
    }
}

struct VoteQ {
    votes: VecDeque<Vote>,
    max_size: usize,
    max_height: usize,
}

impl VoteQ {
    fn new(max_size: usize, max_height: usize) -> Self {
        VoteQ { votes: VecDeque::new(), max_size, max_height}
    }
    fn rollback(&mut self, height: usize) -> usize {
        let num_old = self.votes.iter().take_while(|v| v.lock_height() < vote.height).count();
        for _ in 0..num_old {
            self.votes.pop_front();
        }
        self.max_votes -= num_old;
        num_old
    }
    fn push_front(&self, vote: Vote) {
        assert!(vote.height <= vote.max_height);
        assert!(!self.is_full());
        // double the previous lockouts
        for v in &mut self.votes {
            v.lockout *= 2;
        }
        // push the new vote to the font
        self.votes.push_front(vote); 
    }
    fn pop_back(&mut self) {
        assert!(self.is_full());
        let _ = self.votes.pop_back(); 
    }
    fn is_full(&self) {
        self.votes.len() == MAX_VOTES
    }
    fn last_vote(&self) -> Option<&Vote> {
        self.votes.front()
    }
    fn is_vote_valid(&self, vote: &Vote) -> bool {
        // check that the vote is valid
        for v in &self.votes {
            if !v.branch.is_derived(&vote.branch) && v.lock_height() >= vote.height {
                return false;
            }
        }
        true
    }
    fn merge(&mut self, mut other: VoteQ) {
        self.append(&mut other);
    }
}

pub struct Client {
    queues: Vec<VoteQ>
}

pub const MAX_VOTES: usize = 32usize;
pub const FINALITY_DEPTH: usize = 8;

impl Default for Client {
    fn default() -> Self {
        let mut queues = Vec::new();
        queues.push(VoteQ::new(MAX_VOTES, 1<<MAX_VOTES));
        Client { queues }
    }
}

impl Client {
    fn rollback(&mut self, height: usize) {
        let num_old = self.queues.iter().rev().take_while(|v| v.max_height < height).count();
        for _ in 0..num_old {
            self.queues.pop();
        }
        assert!(self.queues.is_empty());
    }
    fn collapse(&mut self) -> {
        loop {
            if !self.queues.last().is_full() {
                break;
            }
            let full = self.queues.pop();
            self.queues.last().merge(&mut full);
        }
    }
    pub fn accept_vote(&mut self, vote: Vote) -> bool {
        self.rollback(vote.height);
        let num_old = self.queues.last().rollback(vote.height);
        if num_old > 0 {
            queues.push(VoteQ::new(num_old, vote.height + 1<<num_old));
        }
        self.queues.last().push(vote);
        self.collapse();
        if self.queue.last().is_full() {
            self.queue.last().pop_back();
        }
    }

    pub fn last_branch(&self) -> Branch {
        self.queues.last().last_vote().map(|v| v.branch.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use std::cmp;

    fn create_network(sz: usize) -> Vec<Client>  {
        (0..sz).into_iter().map(|_| Client::default()).collect()
    }
    fn create_finality_map(network: &Vec<Client>) -> HashMap<usize, Vec<usize>> {
        let mut fins = HashMap::new();
        for node in network {
            for (i,v) in node.votes.iter().enumerate() {
                let entry = fins.entry(v.branch_id).or_insert(vec![0usize; MAX_VOTES]);
                entry[i] += v.lockout;
            }
        }
        fins
    }
    fn calc_branch_finality(finmap: &HashMap<usize, Vec<usize>>, branch_id: usize) -> Vec<f64> {
        finmap.get(&branch_id).map(|bfins| {
            let totals = calc_totals(finmap);
            totals.iter().zip(bfins).map(|x| (*x.1 as f64)/(*x.0 as f64)).collect()
        }).unwrap_or(vec![1.0; MAX_VOTES])
    }
    fn calc_totals(finmap: &HashMap<usize, Vec<usize>>) -> Vec<usize> {
        let mut totals = vec![0usize; MAX_VOTES];
        for fins in finmap.values() {
            for (i,f) in fins.iter().enumerate() {
                totals[i] += *f;
            }
        }
        totals
    }
    fn calc_maxes(finmap: &HashMap<usize, Vec<usize>>) -> Vec<usize> {
        let mut maxes = vec![0usize; MAX_VOTES];
        for fins in finmap.values() {
            for (i,f) in fins.iter().enumerate() {
                maxes[i] = cmp::max(maxes[i], *f);
            }
        }
        maxes
    }
 
    fn print_max(finmap: &HashMap<usize, Vec<usize>>) -> String {
        let totals = calc_totals(finmap);
        let maxes = calc_maxes(finmap);
        let strs: Vec<String> = maxes.iter().zip(&totals).map(|x| format!("{}", (*x.0 as f64)/(*x.1 as f64))).collect();
        strs.join(" ")
    }
    #[test]
    fn test_partitions() {
        let mut network = create_network(100);
        let num = 3;
        let partition_time = 4;
        let len = network.len();
        for rounds in 0..partition_time {
            for i in 0..network.len() {
                let height = rounds * len + i;
                let branch_id = i % num;
                let vote = Vote::new(branch_id, height);
                let fins = create_finality_map(&network);
                let bfin = calc_branch_finality(&fins, branch_id);
                for (j,node) in network.iter_mut().enumerate() {
                    if j % num == branch_id { 
                        node.accept_vote(&bfin, vote.clone());
                    }
                }
                let fins = create_finality_map(&network);
                println!("{} {}", height, print_max(&fins));
            }
        } 
        let fins = create_finality_map(&network);
        println!("{}", print_max(&fins));
        for rounds in partition_time .. partition_time  {
            for i in 0..network.len() {
                let mut branch_id = network[i].last_branch_id();
                let height = rounds * len + i;
                let vote = Vote::new(branch_id, height);
                let fins = create_finality_map(&network);
                let bfin = calc_branch_finality(&fins, branch_id);
                for node in &mut network {
                    node.accept_vote(&bfin, vote.clone());
                }
                let fins = create_finality_map(&network);
                println!("{} {}", height, print_max(&fins));
            }
        }
    } 
}
