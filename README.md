# LockTower

LockTower is Solana's *Nakomoto Branch Selection* algorithm based on time locks. It satisfies the following properties:

* A voter can eventually recover from voting on a branch that never sees majority votes.
* If the voters share a common ancestor then they will converge to a branch containing that ancestor no matter how they are partitioned.  Although it may not be the latest possible ancestor at the start of the partition.
* Rollback requires exponentially more time for older votes than for newer votes.
* Voters can independently configure a vote threshold they would like to see before committing a vote to a higher lockout.  This allows each voter to make a trade-off of risk and reward.

## Time

For networks like Solana time can be the PoH hash count which is a VDF that provides a source of time before consensus. Other networks adopting this approach would need to consider a global source of time.

## Algorithm

The basic idea to this approach is to stack consensus votes.  Votes in the stack must be for branches that are descendent from each other, and for branches that are valid in the ledger they are submitted to.  Each consensus vote has a `lockout` in unites of time before it can be discarded.  When a vote is added to the stack the lockouts of all the previous votes in the stack are doubled (see #Rollback).  With each new vote a voter commits the previous votes to a ever increasing lockout.  Since at 32 votes we can consider the system to be at `max lockout` any votes with a lockout above 1<<32 are dequeued.  Dequeuing a vote is the trigger for a reward.  If a vote expires before it is dequeued, it and all the votes above it are popped from the vote stack.  The voter needs to start rebuilding the stack from that point.


### Rollback

Before a vote is pushed to the stack, all the votes leading up to vote with a lower lock time than the new vote are popped.  After rollback lockouts are not doubled until the voter catches up to the rollback height of votes.

For example, a vote stack with the following state:

| vote time | lockout | lock expiration time |
|----------:|--------:|---------------------:|
|         4 |      2  |                    6 |
|         3 |      4  |                    7 |
|         2 |      8  |                   10 |
|         1 |      16 |                   17 |

If the next vote is at time 9 the resulting state will be

| vote time | lockout | lock expiration time |
|----------:|--------:|---------------------:|
|         9 |      2  |                   11 |
|         2 |      8  |                   10 |
|         1 |      16 |                   17 |
                              
Next vote is at time 10

| vote time | lockout | lock expiration time |
|----------:|--------:|---------------------:|
|        10 |       2 |                   12 |
|         9 |       4 |                   13 |
|         2 |       8 |                   10 |
|         1 |      16 |                   17 |
                               
At time 10 the new votes caught up to the previous votes.  But the vote created at time 2 expires at 10, so the when next vote at time 11 is applied the entire stack will unroll.

| vote time | lockout | lock expiration time |
|----------:|--------:|---------------------:|
|        11 |       2 |                   13 |
|         1 |      16 |                   17 |

### Slashing and Rewards

The purpose of the lockouts is to force a voter to commit opportunity cost to a specific branch.  Voters that violate the lockouts and vote for a diverging branch within the lockout should be punished.  Slashing or simply freezing the voter from rewards for a long period of time can be used as punishment.

Voters should be rewarded for selecting the branch that the rest of the network selected as often as possible.  This is well aligned with generating a reward when the vote stack is full and the oldest vote needs to be dequeued.  Thus a reward should be generated for each successful dequeue.
 
### Economic Finality

Economic finality can be calculated as the total opportunity costs due to the vote lockout at a given branch.  As a votes get further and further buried the economic finality increases because the cost of unrolling would be the total loss of all the reward from the lockouts at that branch.  

### Thresholds

Each voter can independently set a threshold of network commitment to a branch before that voter commits to a branch.  For example, at vote stack index 7, the lockout is 256 time units.  A voter may withhold votes and let votes 0-7 expire unless the vote at index 7 has at greater than 50% commitment in the network.  This allows each voter to independently control how much risk to commit to a branch.  Committing to a branch faster would allow the voter to earn more rewards.

### Impact of Receive Errors

* with 10% of packet drops, the depth of the trunk is about 77% of the max possible
```
time: 4007, tip converged: 94, trunk id: 4005, trunk time: 4002, trunk converged 100, trunk depth 3121
```
* with 90% of packet drops, the depth of the trunk is about 8.6% of the max possible
```
time: 4007, tip converged: 10, trunk id: 3830, trunk time: 3827, trunk converged 100, trunk depth 348
```

### Simulation
Run with cargo

```
cargo test all_partitions --release -- --nocapture
```

The output will look like this
```
time: 336, tip converged: 76, trunk id: 434, trunk time: 334, trunk converged 98, trunk depth 65
```
* time - The current network time.  Each packet is transmitted to the network at a different time value.
* tip converged - How common is the tip of every voter in the network.
* trunk id - Branch of every trunk.  Every transmission generates a new branch.  A trunk is the newest most common branch for the largest converged set of the network.
* trunk converged - How many voters have converged on this common branch.
* trunk depth - How deep is this branch, or the height of this ledger.

The tests can be configured with the number of starting partitions and the fail rate to transmit blocks to individual voters.
```
    /// * num_partitions - 1 to 100 partitions
    /// * fail_rate - 0 to 1.0 rate of packet receive failure
    fn test_with_partitions(num_partitions: usize, fail_rate: f64)
```

### Algorithm parameters

These parameters need to be tuned.

* Number of votes in the stack before dequeue occurs (32).
* Rate of growth for lockouts in the stack (2x).
* Starting default lockout (2).
* Threshold depth for minimum network commitment before committing to the branch (8).
* Minimum network commitment size at threshold depth (50%+).
