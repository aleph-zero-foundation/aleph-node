# pallet-aleph

This pallet is the runtime companion of the Aleph finality gadget.

Currently, it only provides support for changing sessions but in the future
it will allow reporting equivocation in AlephBFT.

This pallet relies on an extension of the `AlephSessionApi` Runtime API to handle the finality
version. The scheduled version change is persisted as `FinalityScheduledVersionChange`. This
value stores the information about a scheduled finality version change, where `version_incoming`
is the version to be set and `session` is the session on which the new version will be set.
A `pallet_session::Session_Manager` checks whether a scheduled version change has moved into
the past and, if so, records it as the current version represented as `FinalityVersion`,
and clears `FinalityScheduledVersionChange`.
It is always possible to reschedule a version change. In order to cancel a scheduled version
change rather than reschedule it, a new version change should be scheduled with
`version_incoming` set to the current value of `FinalityVersion`.

License: Apache 2.0
