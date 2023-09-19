# pallet-committee-management

## Ban logic
In case of insufficient validator's uptime, we need to remove such validators from
the committee, so that the network is as healthy as possible. This is achieved by calculating
number of _underperformance_ sessions, which means that number of blocks produced by the
validator is less than some predefined threshold.
In other words, if a validator:
* performance in a session is less or equal to a configurable threshold
`BanConfig::minimal_expected_performance` (from 0 to 100%), and,
* it happened at least `BanConfig::underperformed_session_count_threshold` times,
then the validator is considered an underperformer and hence removed (ie _banned out_) from the
committee.

### Thresholds
There are two ban thresholds described above, see [`BanConfig`].

#### Next era vs current era
Current and next era have distinct thresholds values, as we calculate bans during the start of the new era.
They follow the same logic as next era committee seats: at the time of planning the first
session of next the era, next values become current ones.
