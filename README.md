# LockTower

Lock Tower is an implementation of Nakomoto Consensus with time based locks. It satisfies the following properties:

* If the nodes share a common ancestor than they will converge to a branch containing that ancestor no matter how they are partitioned.

* Rollback requires exponentially more time for older votes than for newer votes.

* Nodes can independently configure a vote threshold they would like to see before committing a vote to a higher lockout.  This allows each node to make a trade-off of risk and reward.

## Time

For networks like Solana time can be the PoH hash count which is a VDF that provides a source of time before consensus. Other sources of time can be used as well.

## Algorithm

The basic idea to this approach is to stack consensus votes.  Each consensus vote has a `lockout` before it can be switched.  When a vote is added to the stack the lockouts of all the votes in the stack are doubled.  With each new vote a node commits the previous votes to a ever increasing lockout.  Since at 32 votes we can consider the system to be at `max lockout` any votes with a lockout above 1<<32 are dequeued.  Dequeuing a vote is the trigger for a reward.


### Rollback

Before a vote is pushed to the stack, all the votes leading up to vote with a lower lock time than the new vote are purged.  After rollback lockouts are not doubled until the node catches up to the rollback height of votes.

For example, a vote stack with the following state:

| vote time | lockout | lock time |
|----------:|--------:|----------:|
|         4 |      2  |         6 |
|         3 |      4  |         7 |
|         2 |      8  |        10 |
|         1 |      16 |        17 |

If the next vote is at time 9 the resulting state will be

| vote time | lockout | lock time |
|----------:|--------:|----------:|
|         9 |      2  |        11 |
|         2 |      8  |        10 |
|         1 |      16 |        17 |
                              
Next vote is at time 10

| vote time | lockout | lock time |
|----------:|--------:|----------:|
|        10 |       2 |        12 |
|         9 |       4 |        13 |
|         2 |       8 |        10 |
|         1 |      16 |        17 |
                               
At time 10 the new votes caught up to the previous votes.  But the vote created at time 2 expires at 10, so the when next vote at time 11 is applied the entire stack will unroll.

| vote time | lockout | lock time |
|----------:|--------:|----------:|
|        11 |       2 |        13 |
|         1 |      16 |        17 |

### Slashing and Rewards

The purpose of the lockouts is to force a node to commit time to a specific branch.  Nodes that violate the lockouts and vote for a diverging branch within the lockout should be punished.

Nodes should be rewarded for selecting the right branch with the rest of the network as often as possible.  This is well aligned with generating a reward when the vote stack is full and the oldest vote needs to be dequeued.
 
### Thresholds

Each node can independently set a threshold of network commitment to a branch before that node commits to a branch.  For example, at vote stack index 7, the lockout is 256 time units.  A node may withhold votes and let votes 0-7 expire unless the vote at index 7 has at greater than 50% commitment in the network.  This allows each node to independently control how much risk to commit to a branch.  Committing to a branch faster would allow the node to earn more rewards.

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
* tip converged - How common is the tip of every node in the network.
* trunk id - Branch of every trunk.  Every transmission generates a new branch.  A trunk is the newest most common branch for the largest converged set of the network.
* trunk converged - How many nodes have converged on this common branch.
* trunk depth - How deep is this branch, or the height of this ledger.

The tests can be configured with the number of starting partitions and the fail rate to transmit blocks to individual nodes.
```
    /// * num_partitions - 1 to 100 partitions
    /// * fail_rate - 0 to 1.0 rate of packet receive failure
    fn test_with_partitions(num_partitions: usize, fail_rate: f64)
```
