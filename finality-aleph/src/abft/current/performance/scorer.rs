use current_aleph_bft::{NodeCount, NodeMap, Round};

use crate::{abft::current::performance::Batch, aleph_primitives::RawScore, UnverifiedHeader};

/// Scoring ABFT performance based on returned ordered unit batches.
pub struct Scorer {
    newest_unit_by: NodeMap<Round>,
}

impl Scorer {
    /// Create a new scorer for the provided node count.
    pub fn new(node_count: NodeCount) -> Self {
        Scorer {
            newest_unit_by: NodeMap::with_size(node_count),
        }
    }

    /// Add a batch of ordered units and return a score consisting of numbers of rounds a specific
    /// node is behind.
    pub fn process_batch<UH: UnverifiedHeader>(&mut self, batch: Batch<UH>) -> RawScore {
        let max_round = batch.last().expect("batches always contain a head").round;
        for unit in batch {
            // Units are always added in order, so any unit created by an honest node
            // here has a round greater than any that was included earlier.
            // This is not necessarily true for forkers, but punishing them is fine.
            self.newest_unit_by.insert(unit.creator, unit.round)
        }
        let all_nodes = self.newest_unit_by.size().into_iterator();
        all_nodes
            .map(|node_id| {
                self.newest_unit_by
                    .get(node_id)
                    // All other units have lower round than head, so the saturating_sub is just
                    // subtraction.
                    .map(|unit_round| max_round.saturating_sub(*unit_round))
                    // If we don't have a unit it's the same as having a unit of round equal to -1.
                    .unwrap_or(max_round.saturating_add(1))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use current_aleph_bft::{NodeCount, OrderedUnit, Round};

    use super::Scorer;
    use crate::{block::mock::MockHeader, data_io::AlephData, Hasher};

    const NODE_COUNT: NodeCount = NodeCount(7);

    fn units_up_to(max_round: Round) -> Vec<Vec<OrderedUnit<AlephData<MockHeader>, Hasher>>> {
        let mut result = Vec::new();
        for round in 0..=max_round {
            let mut round_units = Vec::new();
            for creator in NODE_COUNT.into_iterator() {
                round_units.push(OrderedUnit {
                    data: None,
                    // We ignore the parents, so just putting nothing here.
                    parents: Vec::new(),
                    hash: Hasher::random_hash(),
                    creator,
                    round,
                });
            }
            result.push(round_units);
        }
        result
    }

    #[test]
    fn processes_initial_batch() {
        let mut scorer = Scorer::new(NODE_COUNT);
        let unit = units_up_to(0)
            .pop()
            .expect("there is a round")
            .pop()
            .expect("there is a unit");
        assert_eq!(scorer.process_batch(vec![unit]), vec![1, 1, 1, 1, 1, 1, 0]);
    }

    #[test]
    fn processes_perfect_performance_batch() {
        let mut scorer = Scorer::new(NODE_COUNT);
        let mut all_units = units_up_to(1);
        let mut round_one_units = all_units.pop().expect("just created");
        let mut round_zero_units = all_units.pop().expect("just created");
        let first_head = round_zero_units.pop().expect("there is a unit");
        assert_eq!(
            scorer.process_batch(vec![first_head]),
            vec![1, 1, 1, 1, 1, 1, 0]
        );
        let second_head = round_one_units.pop().expect("there is a unit");
        round_zero_units.push(second_head);
        assert_eq!(
            scorer.process_batch(round_zero_units),
            vec![1, 1, 1, 1, 1, 1, 0]
        );
    }

    #[test]
    fn processes_lacking_creator_batch() {
        let mut scorer = Scorer::new(NODE_COUNT);
        let mut all_units = units_up_to(1);
        let mut round_one_units = all_units.pop().expect("just created");
        round_one_units.pop();
        let mut round_zero_units = all_units.pop().expect("just created");
        round_zero_units.pop();
        let first_head = round_zero_units.pop().expect("there is a unit");
        assert_eq!(
            scorer.process_batch(vec![first_head]),
            vec![1, 1, 1, 1, 1, 0, 1]
        );
        let second_head = round_one_units.pop().expect("there is a unit");
        round_zero_units.push(second_head);
        assert_eq!(
            scorer.process_batch(round_zero_units),
            vec![1, 1, 1, 1, 1, 0, 2]
        );
    }

    #[test]
    fn processes_lagging_creator_batch() {
        let mut scorer = Scorer::new(NODE_COUNT);
        let mut all_units = units_up_to(2);
        let mut round_two_units = all_units.pop().expect("just created");
        round_two_units.pop();
        let mut round_one_units = all_units.pop().expect("just created");
        round_one_units.pop();
        let mut round_zero_units = all_units.pop().expect("just created");
        let lagged_unit = round_zero_units.pop().expect("just created");
        let first_head = round_zero_units.pop().expect("there is a unit");
        assert_eq!(
            scorer.process_batch(vec![first_head]),
            vec![1, 1, 1, 1, 1, 0, 1]
        );
        let second_head = round_one_units.pop().expect("there is a unit");
        round_zero_units.push(second_head);
        assert_eq!(
            scorer.process_batch(round_zero_units),
            vec![1, 1, 1, 1, 1, 0, 2]
        );
        let third_head = round_two_units.pop().expect("there is a unit");
        let third_batch = iter::once(lagged_unit)
            .chain(round_one_units)
            .chain(iter::once(third_head))
            .collect();
        assert_eq!(scorer.process_batch(third_batch), vec![1, 1, 1, 1, 1, 0, 2]);
    }
}
